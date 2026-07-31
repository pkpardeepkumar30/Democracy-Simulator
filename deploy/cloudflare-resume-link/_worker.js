const ORIGIN = "https://democracy-game-236938724273.europe-west1.run.app";

export default {
  async fetch(request) {
    const publicUrl = new URL(request.url);
    const originUrl = new URL(`${publicUrl.pathname}${publicUrl.search}`, ORIGIN);
    const headers = new Headers(request.headers);

    // The runtime supplies the origin Host header from originUrl. These headers
    // preserve the public address for any future backend URL generation.
    headers.delete("host");
    headers.set("x-forwarded-host", publicUrl.host);
    headers.set("x-forwarded-proto", publicUrl.protocol.slice(0, -1));

    const originResponse = await fetch(
      new Request(originUrl, {
        method: request.method,
        headers,
        body: request.body,
        redirect: "manual",
      }),
    );

    const responseHeaders = new Headers(originResponse.headers);
    const location = responseHeaders.get("location");

    // Keep same-origin redirects on the short public hostname as well.
    if (location) {
      const redirectUrl = new URL(location, originUrl);
      if (redirectUrl.origin === ORIGIN) {
        responseHeaders.set(
          "location",
          `${publicUrl.origin}${redirectUrl.pathname}${redirectUrl.search}${redirectUrl.hash}`,
        );
      }
    }

    return new Response(originResponse.body, {
      status: originResponse.status,
      statusText: originResponse.statusText,
      headers: responseHeaders,
    });
  },
};
