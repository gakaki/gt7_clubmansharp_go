# Build and deploy to Cloudflare Pages.
# One-time: npx wrangler login   (or set $env:CLOUDFLARE_API_TOKEN)
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
npm run build
npx wrangler pages deploy dist --project-name gt7-cafe --branch main
