// Offline provider responses shared by Worker and real-browser OAuth fixtures.
// Every unmatched outbound request fails instead of reaching a real provider.
export const githubIdentityRedirectTarget = "https://redirected-provider.example.invalid/identity";
export function githubFixtureFetch(subject = "54321", scenario = "success"): typeof fetch {
  return async (input, init) => {
    // Preserve native Workerd option validation before returning offline data.
    const request = new Request(input, init);
    const { url, method, headers } = request;
    if (url === "https://github.com/login/oauth/access_token" && method === "POST") {
      return Response.json({ access_token: "fixture-oauth-access", token_type: "bearer", scope: "user:email" });
    }
    if (method === "GET" && headers.get("authorization") === "Bearer fixture-oauth-access") {
      if (scenario === "profile-unavailable") throw new TypeError("Offline fixture: provider profile fetch failed");
      if (scenario === "profile-read-failure" && url === "https://api.github.com/user") return new Response(new ReadableStream({ start(controller) { controller.error(new TypeError("Offline fixture: profile body read failed")); } }));
      if ((scenario === "profile-redirect" && url === "https://api.github.com/user") || (scenario === "emails-redirect" && url === "https://api.github.com/user/emails")) return new Response(null, { status: 302, headers: { location: githubIdentityRedirectTarget } });
      if (url === "https://api.github.com/user") return Response.json({ id: Number(subject), login: "fixture" });
      if (url === "https://api.github.com/user/emails") return Response.json([{ email: `oauth-${subject}@example.invalid`, primary: true, verified: true }]);
    }
    throw new Error("Unexpected outbound OAuth fixture request");
  };
}
