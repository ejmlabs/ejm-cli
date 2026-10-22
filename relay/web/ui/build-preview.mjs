// Local visual fixture. Build production templates first; no account or host is contacted.
import { cp, mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { templates } from "../.ui/worker.ts";

const source = path.dirname(fileURLToPath(import.meta.url));
const output = process.env.RELAY_PREVIEW_OUT;
if (!output || !path.isAbsolute(output)) throw new Error("Set RELAY_PREVIEW_OUT to a new absolute directory.");
await mkdir(output); // Refuse to replace an existing preview.
const addFixture = (html, mode) => html.replace("<body ", `<body data-preview="${mode}" `)
  .replace("</body>", '<script src="/preview.js" defer></script></body>');
const controller = await readFile(path.join(source, "../.ui/index.html"), "utf8");
const appScript = '<script type="module" src="/control/assets/app.js"></script>';
if (controller.split(appScript).length !== 2) throw new Error("Expected exactly one controller entry script.");
const offlineController = controller.replace(appScript, "");
const account = templates.pageStart + "Relay account" + templates.pageHeading + "Relay account"
  + templates.pageContents + templates.accountStatus + templates.account + templates.pageEnd + templates.pageClose;
for (const [route, html, mode] of [["account", account, "account"], ["control", offlineController, "session"], ["pairing", offlineController, "pairing"]]) {
  await mkdir(path.join(output, route));
  await writeFile(path.join(output, route, "index.html"), addFixture(html, mode));
}
await cp(path.join(source, "../.ui/common"), path.join(output, "common"), { recursive: true });
await cp(path.join(source, "../.ui/notices"), path.join(output, "notices"), { recursive: true });
await writeFile(path.join(output, "preview.js"), String.raw`
const byId=id=>{const node=document.getElementById(id);if(!node)throw new Error('Missing preview element: '+id);return node;};
const sample='Local visual preview · sample data · actions disabled';
document.querySelectorAll('form').forEach(form=>form.addEventListener('submit',event=>event.preventDefault()));
document.querySelectorAll('button,textarea,input,select').forEach(control=>{control.disabled=true;});
document.querySelectorAll('a').forEach(link=>{const url=new URL(link.href,location.href);if(!['/account','/control','/pairing'].some(route=>url.pathname===route||url.pathname.startsWith(route+'/'))&&!url.hash)link.addEventListener('click',event=>{event.preventDefault();byId('status').textContent=sample+'. This review includes Account, Session and Pairing.';});});
byId('status').textContent=sample;
if(document.body.dataset.preview==='account'){
  document.querySelector('[data-field="email"]').textContent='sample@example.invalid';
  document.querySelector('[data-field="termsVersion"]').textContent='beta-v1';
  const terms=byId('accept-terms');const required=new URLSearchParams(location.search).get('terms')==='required';terms.textContent=required?'Accept service terms':'Terms accepted in sample';const disclosure=terms.closest('details');if(disclosure)disclosure.open=required;document.querySelector('[data-field=\"termsState\"]').textContent=required?'Service terms required':'Service terms accepted';
  const plan=document.querySelector('[data-field="plan"]');plan.textContent='Free beta';
  function row(target,label,action){const item=byId('account-row-template').content.firstElementChild.cloneNode(true);item.querySelector('[data-row-title]').textContent=label;const button=item.querySelector('[data-row-action]');if(action){button.textContent=action;button.disabled=true;}else button.parentElement.remove();target.replaceChildren(item);}
  row(byId('devices'),'Sample Mac · CLI host','Revoke');row(byId('rooms'),'Sample relay session · sharing enabled','Stop sharing');
}else{
  if(document.body.dataset.preview==='session'){
    byId('pair-panel').hidden=true;byId('peer-panel').hidden=false;
    byId('connection-state').textContent='Sample · idle';byId('session-heading').textContent='Relay workspace';byId('session-context').textContent='~/projects/relay-demo';
    const identity=byId('session-id');identity.textContent=' · session demo';identity.hidden=false;byId('peer-session-id').textContent='demo-session-visual-preview';
    byId('role-note').textContent='Sample · control access';byId('access').textContent='Sample control access';
    byId('host-fingerprint').textContent='0000 1111 2222 3333 4444 5555 6666 7777 8888 9999 aaaa bbbb cccc dddd eeee ffff';
    byId('controller-fingerprint').textContent='ffff eeee dddd cccc bbbb aaaa 9999 8888 7777 6666 5555 4444 3333 2222 1111 0000';
    byId('share-expiry').textContent='Sample only';byId('action-status').textContent='Sample session. No prompt is sent and no command runs.';
    const transcript=byId('transcript');transcript.replaceChildren();
    for(const [label,title,text]of [['Prompt','you','Simplify this relay workspace. Keep the session in focus.'],['Assistant','ejm-cli','I’ll keep one transcript and one composer, with connection details a step away.'],['Tool','running · read_file','relay/web/ui/screens.tsx'],['Tool result','done · read_file','Read the screen layout and shared control definitions.'],['Assistant','ejm-cli','The account page now leads with your sessions and devices. Plan, terms and account settings sit below.\n\nThe controller follows the same layout as the CLI.'],['Run finished','done','Sample transcript for visual review.']]){
      const entry=byId('transcript-entry-template').content.firstElementChild.cloneNode(true);entry.dataset.kind=label==='Prompt'?'prompt':label==='Assistant'?'assistant':'output';entry.querySelector('[data-terminal-label]').textContent=title;entry.querySelector('[data-terminal-text]').textContent=text;transcript.append(entry);
    }
  }else{byId('connection-state').textContent='Sample · not paired';byId('action-status').textContent='Pairing review only. No invitation is accepted.';}
}
`);
console.log(JSON.stringify({ output, scope: "Static local visual fixture using production templates and assets; no auth, WebSocket, WASM or execution." }));
