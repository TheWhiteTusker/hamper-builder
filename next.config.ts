import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: process.env.BUILD_STANDALONE === "true" ? "standalone" : undefined,
  images: {
    remotePatterns: [
      {
        protocol: "https",
        hostname: "*.supabase.co",
      },
      {
        protocol: "https",
        hostname: "hoftbldhndnngjfjsqhs.supabase.co",
      },
    ],
  },
  experimental: {
    // The spreadsheet import posts six parsed CSVs in one server action.
    serverActions: { bodySizeLimit: "15mb" },
    // proxy.ts buffers every request body and silently truncates past this
    // (default 10mb), which breaks large uploads. Keep it at the action limit.
    proxyClientMaxBodySize: "15mb",
    // Revisiting a page within 2 min is instant and costs no Worker request.
    // Server actions' revalidatePath still clears this, so your own edits show
    // at once; other users' edits show within 2 min.
    staleTimes: { dynamic: 120 },
  },
};

export default nextConfig;
