const STATIC_FILE =
  /\.(?:avif|jpe?g|png|webp|gif|ico|jxl|svg|css|js|json|csv|md|woff2?|otf|ttf)$/i

export default {
  async fetch(request, env) {
    const url = new URL(request.url)
    if (
      (request.method === 'GET' || request.method === 'HEAD') &&
      !STATIC_FILE.test(url.pathname)
    ) {
      return env.ASSETS.fetch(new URL('/index.html', url), request)
    }

    const response = await env.ASSETS.fetch(request)
    if (response.status !== 404) return response

    return new Response('Not found', {
      status: 404,
      headers: {
        'cache-control': 'no-store',
        'content-type': 'text/plain; charset=utf-8',
      },
    })
  },
}
