import { UMAMI_PROXY_PREFIX, proxyUmamiRequest } from "./proxy";

export interface Env {
  UMAMI_ORIGIN: string;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    if (!url.pathname.startsWith(UMAMI_PROXY_PREFIX)) {
      return new Response("Not found", { status: 404 });
    }
    return proxyUmamiRequest(request, env.UMAMI_ORIGIN);
  },
};
