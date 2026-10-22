import { env } from "cloudflare:workers";
import { githubFixtureFetch } from "./github.js";

// Fixture entry only. Production entry/config never imports this provider mock.
globalThis.fetch = githubFixtureFetch("54321", (env as { OAUTH_FIXTURE_SCENARIO?: string }).OAUTH_FIXTURE_SCENARIO);
export { default, Room, AccountQuota, PrivacyAuthority } from "../../src/index.js";
