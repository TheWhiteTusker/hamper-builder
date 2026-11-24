const { app, BrowserWindow, dialog } = require("electron");
const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");
const page = require("./page.cjs");

// Releases are published to the website (pnpm publish:desktop). On launch, and
// from "Check for updates", the app asks before downloading, shows progress,
// then offers Install now / Later — so nobody reinstalls by hand.
const UPDATE_SERVERS = [
  "https://hamper-builder.latticelane.workers.dev/updates/",
  "https://lattice-lane-hamper-builder.latticelane.workers.dev/updates/",
  "https://lattice-lane-hamper-builder.digital-9f6.workers.dev/updates/",
];

async function fetchFromUpdates(relPath) {
  for (const base of UPDATE_SERVERS) {
    try {
      const res = await fetch(base + relPath, { cache: "no-store" });
      if (res.ok) return { res, base };
    } catch {
      // try next server
    }
  }
  return null;
}

const isNewer = (a, b) => {
  const [x, y] = [a, b].map((v) => v.split(".").map(Number));
  for (let i = 0; i < 3; i++) if ((x[i] || 0) !== (y[i] || 0)) return (x[i] || 0) > (y[i] || 0);
  return false;
};

// A downloaded-but-not-installed update waits here until the user picks
// "Install now"; every launch asks again until they do.
const UPDATE_DIR = () => path.join(app.getPath("userData"), "updates");
const PENDING = () => path.join(UPDATE_DIR(), "pending.json");

function readPending() {
  try {
    const p = JSON.parse(fs.readFileSync(PENDING(), "utf8"));
    if (isNewer(p.version, app.getVersion()) && fs.existsSync(p.file)) return p;
  } catch {
    // no pending update
  }
  // Already installed, or never downloaded: clear any leftover installer.
  fs.rmSync(UPDATE_DIR(), { recursive: true, force: true });
  return null;
}

async function askInstall(p) {
  const { response } = await dialog.showMessageBox(getWin(), {
    type: "info",
    buttons: ["Install now", "Later"],
    defaultId: 0,
    cancelId: 1,
    title: "Update ready",
    message: `Lattice Lane ${p.version} is downloaded and ready to install.`,
    detail: "Install now restarts the app on the new version. Later asks again the next time you open the app.",
  });
  if (response !== 0) return { status: "deferred", version: p.version };
  // Silent install over the existing one, then relaunch.
  spawn(p.file, ["/S", "--force-run"], { detached: true, stdio: "ignore" }).unref();
  app.quit();
  return { status: "updating", version: p.version };
}

async function download(base, version) {
  const res = await fetch(base + "Lattice-Lane-Setup.exe", { cache: "no-store" });
  if (!res.ok) throw new Error(`Download failed (${res.status})`);
  const total = Number(res.headers.get("content-length")) || 0;

  fs.mkdirSync(UPDATE_DIR(), { recursive: true });
  const file = path.join(UPDATE_DIR(), `Lattice-Lane-Setup-${version}.exe`);
  const win = new BrowserWindow({
    parent: getWin(),
    modal: true,
    width: 440,
    // Fits the card (40px padding, heading, version, bar, %) without a scrollbar.
    height: 300,
    frame: false,
    resizable: false,
    minimizable: false,
  });
  await win.loadURL(
    page(
      "Downloading update",
      `<h2>Downloading update</h2><p>Lattice Lane ${version}</p>
       <div style="background:#e8e6e1;border-radius:4px;height:8px;overflow:hidden">
         <div id="bar" style="background:#54655b;height:8px;width:0"></div></div>
       <p id="pct">0%</p>`,
    ),
  );

  const out = fs.createWriteStream(file);
  let got = 0;
  let shown = null;
  try {
    for await (const chunk of res.body) {
      got += chunk.length;
      if (!out.write(chunk)) await new Promise((r) => out.once("drain", r));
      const label = total ? `${Math.floor((got / total) * 100)}%` : `${Math.round(got / 1e6)} MB`;
      if (label !== shown) {
        shown = label;
        getWin()?.setProgressBar(total ? got / total : 2);
        win.webContents
          .executeJavaScript(
            `document.getElementById("pct").textContent=${JSON.stringify(label)};` +
              (total ? `document.getElementById("bar").style.width=${JSON.stringify(label)};` : ""),
          )
          .catch(() => {});
      }
    }
    await new Promise((resolve, reject) => out.end((err) => (err ? reject(err) : resolve())));
    if (total && got !== total) throw new Error("Download was incomplete");
  } catch (error) {
    out.destroy();
    fs.rmSync(file, { force: true });
    throw error;
  } finally {
    win.destroy();
    getWin()?.setProgressBar(-1);
  }

  const p = { version, file };
  fs.writeFileSync(PENDING(), JSON.stringify(p));
  return p;
}

let checking = false;

async function checkForUpdate(manual = false) {
  if (!app.isPackaged && !manual) return { status: "dev" };
  if (checking) return { status: "busy" };
  checking = true;
  try {
    const pending = readPending();
    if (pending) return await askInstall(pending);

    const updateResult = await fetchFromUpdates("latest.json");
    if (!updateResult) {
      if (manual) {
        await dialog.showMessageBox(getWin(), {
          type: "error",
          title: "Update Check Failed",
          message: "Could not reach update server. Please check your internet connection.",
        });
      }
      return { status: "error", message: "Could not reach update server" };
    }
    const { version } = await updateResult.res.json();
    if (!isNewer(version, app.getVersion())) {
      if (manual) {
        await dialog.showMessageBox(getWin(), {
          type: "info",
          title: "Up to date",
          message: `You are on the latest version of Lattice Lane (v${app.getVersion()}).`,
        });
      }
      return { status: "up-to-date", version };
    }

    const { response } = await dialog.showMessageBox(getWin(), {
      type: "question",
      buttons: ["Update", "Not now"],
      defaultId: 0,
      cancelId: 1,
      title: "Update available",
      message: `Lattice Lane ${version} is available (you have ${app.getVersion()}).`,
      detail: "Would you like to update?",
    });
    if (response !== 0) return { status: "deferred", version };

    return await askInstall(await download(updateResult.base, version));
  } catch (error) {
    // Offline or a bad download: keep working, try again next launch.
    console.error("Update failed:", error);
    if (manual) {
      await dialog.showMessageBox(getWin(), {
        type: "error",
        title: "Update failed",
        message: `The update could not be completed: ${error.message}`,
        detail: "Check your internet connection and try again.",
      });
    }
    return { status: "error", message: error.message };
  } finally {
    checking = false;
  }
}

// getWin: the main window is recreated, so updates look it up each time.
let getWin = () => null;

module.exports = (windowGetter) => {
  getWin = windowGetter;
  return checkForUpdate;
};
