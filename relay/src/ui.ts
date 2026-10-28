import { templates } from "../web/.ui/worker.js";
import { htmlEscape } from "./http.js";

export { templates };

/** Static shared markup, with escaped product text. No client rendering framework. */
export function documentHtml(title: string, contents: string, accountScript = false): string {
  return `${templates.pageStart}${htmlEscape(title)}${templates.pageHeading}${htmlEscape(title)}${templates.pageContents}${accountScript ? templates.accountStatus : ""}${contents}${templates.pageEnd}${accountScript ? templates.accountScript : ""}${templates.pageClose}`;
}

export function uiPage(title: string, contents: string, status = 200): Response {
  return new Response(documentHtml(title, contents, true), { status, headers: { "content-type": "text/html;charset=utf-8" } });
}

/** The caller selects its section from the route, never from page copy. */
export function withCurrentNav(response: Response, section: "/account" | "/control" | "/privacy"): Response {
  return pageValues(response, {}, {}, { [`header nav a[href="${section}"]`]: { "aria-current": "page" } });
}

/** Values are text by default. HTML is accepted only from build-generated fragments. */
export function pageValues(response: Response, values: Record<string, string>, fragments: Record<string, string> = {}, attributes: Record<string, Record<string, string>> = {}): Response {
  const rewriter = new HTMLRewriter();
  for (const [name, value] of Object.entries(values)) rewriter.on(`[data-field="${name}"]`, { element(element) { element.setInnerContent(value); } });
  for (const [name, value] of Object.entries(fragments)) rewriter.on(`[data-field="${name}"]`, { element(element) { element.setInnerContent(value, { html: true }); } });
  // The platform escapes quotes, not ampersands; encode for one browser parse.
  for (const [selector, fields] of Object.entries(attributes)) rewriter.on(selector, { element(element) { for (const [name, value] of Object.entries(fields)) element.setAttribute(name, htmlEscape(value)); } });
  return rewriter.transform(response);
}
