declare module '*.md?doc' {
  const Page: import('react').ComponentType;
  export default Page;
}
declare module 'virtual:doc-routes' {
  import type { RouteDef } from '@ejm/ssg';
  export const docRoutes: RouteDef[];
}

declare module 'virtual:doc-navigation' {
  const pages: readonly import('@ejm/ui/docs').DocNavigationPage[];
  export default pages;
}
