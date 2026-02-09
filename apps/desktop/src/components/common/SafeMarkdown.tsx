/**
 * Sanitized Markdown renderer for AI-generated content.
 *
 * Uses rehype-sanitize to prevent XSS attacks from untrusted markdown.
 * Only allows http/https links, removes javascript: and data: URIs.
 * Supports GFM tables via remark-gfm.
 */

import ReactMarkdown from 'react-markdown';
import rehypeSanitize, { defaultSchema } from 'rehype-sanitize';
import remarkGfm from 'remark-gfm';
import { open } from '@tauri-apps/plugin-shell';

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
  // Allow table elements for GFM tables
  tagNames: [
    ...(defaultSchema.tagNames || []),
    'table',
    'thead',
    'tbody',
    'tr',
    'th',
    'td',
  ],
  // Remove potentially dangerous attributes
  attributes: {
    ...defaultSchema.attributes,
    '*': ['className', 'id'],
    a: ['href', 'title', 'target', 'rel'],
    img: ['src', 'alt', 'title', 'width', 'height'],
    code: ['className'],
    // Table alignment attributes
    th: ['align', 'scope'],
    td: ['align'],
  },
};

/**
 * Removes any remaining command tags from AI response content.
 * Defense in depth: catches tags that weren't properly parsed by the backend.
 * Uses brace-counting to correctly handle JSON with nested brackets.
 */
function sanitizeCommandTags(content: string): string {
  // Pattern: [[COMMAND: followed by JSON object
  const commandStart = /\[\[[A-Z_]+:\{/g;
  let result = content;
  let match;

  // Process each command tag by counting braces
  while ((match = commandStart.exec(result)) !== null) {
    const startIdx = match.index;
    const jsonStart = startIdx + match[0].length - 1; // Position of opening {

    // Count braces to find matching }
    let braceCount = 1;
    let jsonEnd = -1;

    for (let i = jsonStart + 1; i < result.length && braceCount > 0; i++) {
      if (result[i] === '{') braceCount++;
      else if (result[i] === '}') {
        braceCount--;
        if (braceCount === 0) {
          jsonEnd = i;
          break;
        }
      }
    }

    if (jsonEnd === -1) break; // Malformed, stop processing

    // Find closing brackets (] or ]])
    let endIdx = jsonEnd + 1;
    while (endIdx < result.length && result[endIdx] === ']') {
      endIdx++;
    }

    // Remove this command tag
    result = result.slice(0, startIdx) + result.slice(endIdx);
    commandStart.lastIndex = startIdx; // Reset to check for more tags
  }

  return result.trim();
}

export function SafeMarkdown({ children, className }: SafeMarkdownProps) {
  // Sanitize command tags before rendering (defense in depth)
  const sanitizedContent = sanitizeCommandTags(children);
  const content = (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      rehypePlugins={[[rehypeSanitize, sanitizeSchema]]}
      components={{
        a: ({ href, children }) => (
          <button
            onClick={(e) => {
              e.preventDefault();
              if (href && (href.startsWith('http://') || href.startsWith('https://') || href.startsWith('mailto:'))) {
                open(href);
              }
            }}
            className="text-primary hover:underline cursor-pointer"
          >
            {children}
          </button>
        ),
      }}
    >
      {sanitizedContent}
    </ReactMarkdown>
  );

  // Wrap in div if className is provided (react-markdown v10 doesn't support className prop)
  if (className) {
    return <div className={className}>{content}</div>;
  }

  return content;
}
