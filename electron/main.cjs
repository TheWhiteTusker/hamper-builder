const { app, BrowserWindow, dialog, shell, utilityProcess, ipcMain } = require("electron");
const path = require("path");
const http = require("http");
const net = require("net");
const fs = require("fs");

// The app runs its own Next server on localhost and talks to the same Supabase
// project as the website, so the data is identical but no click waits on the
// Cloudflare Worker (~1.5s per page when measured).
let mainWindow = null;
let server = null;
let appUrl = null;

const page = require("./page.cjs");
const checkForUpdate = require("./updater.cjs")(() => mainWindow);

function freePort() {
  return new Promise((resolve, reject) => {
    const s = net.createServer();
    s.listen(0, "127.0.0.1", () => {
      const { port } = s.address();
      s.close(() => resolve(port));
    });
    s.on("error", reject);
  });
}

function waitForServer(url, timeoutMs = 30000) {
  const start = Date.now();
  return new Promise((resolve, reject) => {
    const attempt = () =>
      http
        .get(url, (res) => {
          res.resume();
          resolve();
        })
        .on("error", () => {
          if (Date.now() - start > timeoutMs) reject(new Error("Local server did not start"));
          else setTimeout(attempt, 100);
        });
    attempt();
  });
}

function serverScript() {
  const packaged = path.join(process.resourcesPath, "standalone", "server.js");
  if (fs.existsSync(packaged)) return packaged;
  return path.join(__dirname, "..", ".next", "standalone", "server.js");
}

async function startServer() {
  const port = await freePort();
  const script = serverScript();
  server = utilityProcess.fork(script, [], {
    cwd: path.dirname(script),
    env: { ...process.env, LATTICE_DESKTOP: "1", PORT: String(port), HOSTNAME: "127.0.0.1", NODE_ENV: "production" },
  });
  server.on("exit", () => {
    server = null;
    if (!app.isQuitting) app.quit();
  });
  appUrl = `http://127.0.0.1:${port}`;
  await waitForServer(appUrl);
}

ipcMain.handle("check-for-updates", async () => checkForUpdate(true));

function createWindow() {
  const ico = path.join(__dirname, "..", "build", "icon.ico");
  mainWindow = new BrowserWindow({
    width: 1440,
    height: 900,
    minWidth: 1024,
    minHeight: 700,
    title: "Lattice Lane - Hamper Builder",
    icon: fs.existsSync(ico) ? ico : path.join(__dirname, "..", "src", "app", "icon.png"),
    autoHideMenuBar: true,
    webPreferences: {
      preload: path.join(__dirname, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      spellcheck: true,
    },
  });

  // Anything that isn't the app itself opens in the default browser.
  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    if (appUrl && url.startsWith(appUrl)) return { action: "allow" };
    shell.openExternal(url);
    return { action: "deny" };
  });

  mainWindow.webContents.on("did-fail-load", (_e, code, _desc, url) => {
    if (code === -3 || !url.startsWith("http")) return; // aborted / redirected
    mainWindow.loadURL(
      page(
        "Lattice Lane - Offline",
        `<h2>Connection Unavailable</h2>
         <p>Could not reach the Lattice Lane database. Check your internet connection and try again.</p>
         <button onclick="location.href='${url}'">Retry</button>`,
      ),
    );
  });

  // F5 reloads like a browser; the hidden default menu only has Ctrl+R.
  mainWindow.webContents.on("before-input-event", (event, input) => {
    if (input.type === "keyDown" && input.key === "F5") {
      event.preventDefault();
      mainWindow.webContents.reload();
    }
  });

  mainWindow.on("closed", () => {
    mainWindow = null;
  });

  // Show something at once; the local server takes a moment on first launch.
  mainWindow.loadURL(page("Lattice Lane", "<h2>Lattice Lane</h2><p>Starting…</p>"));
}

if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on("second-instance", () => {
    if (!mainWindow) return;
    if (mainWindow.isMinimized()) mainWindow.restore();
    mainWindow.focus();
  });

  app.whenReady().then(async () => {
    createWindow();
    try {
      await startServer();
      mainWindow?.loadURL(appUrl);
      checkForUpdate();
    } catch (error) {
      mainWindow?.loadURL(page("Lattice Lane - Error", `<h2>Could not start</h2><p>${error.message}</p>`));
    }
  });

  app.on("before-quit", () => {
    app.isQuitting = true;
    server?.kill();
  });

  app.on("window-all-closed", () => app.quit());
}
