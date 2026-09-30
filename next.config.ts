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
    // Revisiting a page within 30s is instant. Server actions' revalidatePath
    // still clears this, so your own edits show at once.
    staleTimes: { dynamic: 30 },
  },
};

export default nextConfig;
