import { Static } from '@ejm/ssg';
import {
  DocsPage,
  docNavigation,
  type ClientDocPage,
  type DocsPageProps,
} from '@ejm/ui/docs';
import navigation from 'virtual:doc-navigation';

/** Product routes and page order stay local; the shared package owns the reading UI. */
export function DocPageView({
  page,
  renderProse,
  renderCode,
  renderBlock,
}: {
  page: ClientDocPage;
  renderProse?: DocsPageProps['renderProse'];
  renderCode?: DocsPageProps['renderCode'];
  renderBlock?: DocsPageProps['renderBlock'];
}) {
  return (
    <DocsPage
      page={page}
      {...docNavigation(navigation, page.slug)}
      sidebarLabel="Docs pages"
      staticBoundary={Static}
      renderProse={renderProse}
      renderCode={renderCode}
      renderBlock={renderBlock}
    />
  );
}
