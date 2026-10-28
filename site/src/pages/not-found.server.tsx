// Static recovery copy stays in the built 404 document.
import { Band, SectionHead } from '@ejm/ui/layout';
import { InlineLink } from '@ejm/ui/layout/InlineLink';

export default function NotFoundContent() {
  return (
    <Band>
      <SectionHead
        as="h1"
        kicker="404"
        title="This page is not here"
        purpose="page"
      >
        <InlineLink href="/">Back to the home page</InlineLink>
      </SectionHead>
    </Band>
  );
}
