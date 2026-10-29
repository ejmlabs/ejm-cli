export interface DocEntry {
  /** The route, as served: `/docs/user-guide/`. */
  path: string;
  /** Markdown source, relative to the repository root. */
  file: string;
  group: string;
  /** Sidebar text when it differs from the title. */
  label?: string;
  title: string;
  lead: string;
  description: string;
}
export const GUIDES: string;
export const REFERENCE: string;
export const docPages: DocEntry[];
