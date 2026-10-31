import { Hero } from '@ejm/ui/layout/Hero';
import { InlineLink } from '@ejm/ui/layout/InlineLink';

export default function NotFound() {
  return (
    <Hero
      eyebrow="404"
      title="This page is not here"
      actions={<InlineLink href="/">Back to the home page</InlineLink>}
    />
  );
}
