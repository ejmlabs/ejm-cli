import { defineSite } from '@ejm/ssg';
import { siteDefaults } from '@ejm/site/head';
import { docRoutes } from 'virtual:doc-routes';
import NotFound from './pages/NotFound';
import { App } from './shell';
import './styles.css';

export default defineSite({
  ...siteDefaults('cli', {
    description: 'An open source Rust terminal coding agent under MIT. Build from source, connect a model, or watch the recorded example.',
  }),
  App,
  routes: [{ path: '/', load: () => import('./pages/Home'), head: { title: 'Open source terminal coding agent' } }, ...docRoutes],
  notFound: { component: NotFound, head: { title: 'Page not found' } },
});
