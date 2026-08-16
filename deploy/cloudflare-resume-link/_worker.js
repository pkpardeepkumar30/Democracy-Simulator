const ORIGIN = "https://democracy-game-236938724273.europe-west1.run.app";
const PUBLIC_ORIGIN = "https://the-republic.pages.dev";
const VERIFICATION_TAG = '<meta name="google-site-verification" content="i3KkC0aZF8NUh1zQPaxYIZVNnvgiJY7iqLnGn6RDj08" />';

export default {
  async fetch(request) {
    const publicUrl = new URL(request.url);

    if (publicUrl.pathname === "/robots.txt") {
      return new Response(
        `User-agent: *\nAllow: /\nSitemap: ${PUBLIC_ORIGIN}/sitemap.xml\n`,
        { headers: { "content-type": "text/plain; charset=utf-8" } },
      );
    }

    if (publicUrl.pathname === "/sitemap.xml") {
      return new Response(
        `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n  <url>\n    <loc>${PUBLIC_ORIGIN}/</loc>\n    <changefreq>weekly</changefreq>\n    <priority>1.0</priority>\n  </url>\n</urlset>\n`,
        { headers: { "content-type": "application/xml; charset=utf-8" } },
      );
    }

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

    if ((responseHeaders.get("content-type") || "").includes("text/html")) {
      let html = (await originResponse.text()).replaceAll(ORIGIN, PUBLIC_ORIGIN);
      const headTags = [];
      if (!html.includes('name="google-site-verification"')) {
        headTags.push(VERIFICATION_TAG);
      }
      if (!html.includes('name="robots"')) {
        headTags.push('<meta name="robots" content="index,follow,max-image-preview:large" />');
      }
      if (!html.includes('rel="canonical"')) {
        headTags.push(`<link rel="canonical" href="${PUBLIC_ORIGIN}/" />`);
      }
      if (!html.includes('property="og:url"')) {
        headTags.push(`<meta property="og:url" content="${PUBLIC_ORIGIN}/" />`);
      }
      if (headTags.length) {
        html = html.replace("</head>", `  ${headTags.join("\n  ")}\n</head>`);
      }
      responseHeaders.delete("content-length");
      responseHeaders.delete("content-encoding");
      return new Response(html, {
        status: originResponse.status,
        statusText: originResponse.statusText,
        headers: responseHeaders,
      });
    }

    return new Response(originResponse.body, {
      status: originResponse.status,
      statusText: originResponse.statusText,
      headers: responseHeaders,
    });
  },
};
