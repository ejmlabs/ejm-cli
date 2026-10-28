import { createLocalFixture } from "./local-fixture.js";
const fixture = await createLocalFixture();
try {
  const response = await fetch(fixture.origin + "/v1/account", { headers: { cookie: fixture.cookie } });
  const account = await response.json() as { deviceId?: string };
  if (response.status !== 200 || !account.deviceId) throw new Error("cookie_fixture_failed");
  console.log(JSON.stringify({ fixture: "ephemeral_workerd", accountStatus: response.status, deviceRegistered: true, loopback: new URL(fixture.origin).hostname }));
} finally { await fixture.dispose(); }
