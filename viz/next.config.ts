import type { NextConfig } from 'next';

const nextConfig: NextConfig = {
  output: 'export',
  images: { unoptimized: true },
  // During `next dev`, proxy /api/* to the Rust server
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: `http://localhost:${process.env.KERRIGAN_PORT ?? '7070'}/api/:path*`,
      },
    ];
  },
};

export default nextConfig;
