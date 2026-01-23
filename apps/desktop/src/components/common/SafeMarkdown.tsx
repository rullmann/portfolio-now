/**
 * Sanitized Markdown renderer for AI-generated content.
 *
 * Uses rehype-sanitize to prevent XSS attacks from untrusted markdown.
 * Only allows http/https links, removes javascript: and data: URIs.
 */

import ReactMarkdown from 'react-markdown';
import rehypeSanitize, { defaultSchema } from 'rehype-sanitize';

interface SafeMarkdownProps {
  children: string;
  className?: string;
}

// Custom schema: only allow safe protocols for links
const sanitizeSchema = {
  ...defaultSchema,
  protocols: {
    ...defaultSchema.protocols,
    href: ['http', 'https', 'mailto'],
    src: ['http', 'https'],
  },
  // Remove potentially dangerous attributes
  attributes: {
    ...defaultSchema.attributes,
    '*': ['className', 'id'],
    a: ['href', 'title', 'target', 'rel'],
    img: ['src', 'alt', 'title', 'width', 'height'],
    code: ['className'],
  },
};

export function SafeMarkdown({ children, className }: SafeMarkdownProps) {
  const content = (
    <ReactMarkdown rehypePlugins={[[rehypeSanitize, sanitizeSchema]]}>
      {children}
    </ReactMarkdown>
  );

  // Wrap in div if className is provided (react-markdown v10 doesn't support className prop)
  if (className) {
    return <div className={className}>{content}</div>;
  }

  return content;
}
