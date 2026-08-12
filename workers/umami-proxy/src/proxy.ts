/**
 * First-party Umami proxy for gg.chtnnhfoundation.org (/stats/*).
 * Mirrors kickstart's worker/umami-proxy.ts.
 */
export const UMAMI_PROXY_PREFIX = "/stats";
export const DEFAULT_UMAMI_ORIGIN = "https://umami.chtnnhfoundation.org";

export function buildUmamiTargetUrl(
  pathname: string,
  origin: string,
): URL | null {
  if (!pathname.startsWith(UMAMI_PROXY_PREFIX)) return null;
  const rest = pathname.slice(UMAMI_PROXY_PREFIX.length) || "/";
  const base = origin.endsWith("/") ? origin : `${origin}/`;
  return new URL(rest.startsWith("/") ? rest.slice(1) : rest, base);
}

export async function proxyUmamiRequest(
  request: Request,
  origin: string,
): Promise<Response> {
  const url = new URL(request.url);
  const target = buildUmamiTargetUrl(url.pathname, origin);
  if (!target) {
    return new Response("Not found", { status: 404 });
  }
  target.search = url.search;

  const headers = new Headers();
  const umamiHost = new URL(origin).host;
  headers.set("Host", umamiHost);

  const contentType = request.headers.get("Content-Type");
  if (contentType) headers.set("Content-Type", contentType);

  const clientIp = request.headers.get("CF-Connecting-IP");
  if (clientIp) headers.set("X-Forwarded-For", clientIp);
  headers.set("X-Forwarded-Proto", url.protocol.replace(":", ""));
  headers.set("X-Forwarded-Host", url.host);

  const upstream = await fetch(target.toString(), {
    method: request.method,
    headers,
    body:
      request.method !== "GET" && request.method !== "HEAD"
        ? request.body
        : undefined,
  });

  const responseHeaders = new Headers(upstream.headers);
  if (url.pathname.endsWith("/script.js")) {
    responseHeaders.set("Cache-Control", "public, max-age=3600");
  }

  return new Response(upstream.body, {
    status: upstream.status,
    statusText: upstream.statusText,
    headers: responseHeaders,
  });
}
