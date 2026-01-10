/**
 * Official AI Provider logos as SVG components.
 * Logos sourced from official brand resources.
 */

interface LogoProps {
  size?: number;
  className?: string;
}

/**
 * Anthropic Claude logo
 */
export function ClaudeLogo({ size = 24, className = '' }: LogoProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      {/* Anthropic Claude wordmark "A" stylized */}
      <path
        d="M17.304 3H14.432L20 21H22.872L17.304 3Z"
        fill="#D97757"
      />
      <path
        d="M6.696 3L1.128 21H4L5.392 16.5H12.608L14 21H16.872L11.304 3H6.696ZM6.288 14.0625L9 5.4375L11.712 14.0625H6.288Z"
        fill="#D97757"
      />
    </svg>
  );
}

/**
 * OpenAI logo
 */
export function OpenAILogo({ size = 24, className = '' }: LogoProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      <path
        d="M22.2819 9.8211a5.9847 5.9847 0 0 0-.5157-4.9108 6.0462 6.0462 0 0 0-6.5098-2.9A6.0651 6.0651 0 0 0 4.9807 4.1818a5.9847 5.9847 0 0 0-3.9977 2.9 6.0462 6.0462 0 0 0 .7427 7.0966 5.98 5.98 0 0 0 .511 4.9107 6.051 6.051 0 0 0 6.5146 2.9001A5.9847 5.9847 0 0 0 13.2599 24a6.0557 6.0557 0 0 0 5.7718-4.2058 5.9894 5.9894 0 0 0 3.9977-2.9001 6.0557 6.0557 0 0 0-.7475-7.0729zm-9.022 12.6081a4.4755 4.4755 0 0 1-2.8764-1.0408l.1419-.0804 4.7783-2.7582a.7948.7948 0 0 0 .3927-.6813v-6.7369l2.02 1.1686a.071.071 0 0 1 .038.052v5.5826a4.504 4.504 0 0 1-4.4945 4.4944zm-9.6607-4.1254a4.4708 4.4708 0 0 1-.5346-3.0137l.142.0852 4.783 2.7582a.7712.7712 0 0 0 .7806 0l5.8428-3.3685v2.3324a.0804.0804 0 0 1-.0332.0615L9.74 19.9502a4.4992 4.4992 0 0 1-6.1408-1.6464zM2.3408 7.8956a4.485 4.485 0 0 1 2.3655-1.9728V11.6a.7664.7664 0 0 0 .3879.6765l5.8144 3.3543-2.0201 1.1685a.0757.0757 0 0 1-.071 0l-4.8303-2.7865A4.504 4.504 0 0 1 2.3408 7.8956zm16.0993 3.8558L12.6 8.3829l2.02-1.1638a.0757.0757 0 0 1 .071 0l4.8303 2.7913a4.4944 4.4944 0 0 1-.6765 8.1042v-5.6772a.79.79 0 0 0-.4043-.6906zm2.0107-3.0231l-.142-.0852-4.7735-2.7818a.7759.7759 0 0 0-.7854 0L9.409 9.2297V6.8974a.0662.0662 0 0 1 .0284-.0615l4.8303-2.7866a4.4992 4.4992 0 0 1 6.1408 1.6465 4.4708 4.4708 0 0 1 .5765 3.0137zM8.3036 12.0453l-2.02-1.1638a.0804.0804 0 0 1-.038-.0567V5.2694a4.4992 4.4992 0 0 1 7.3757-3.4537l-.142.0805L8.704 4.6648a.7948.7948 0 0 0-.3927.6813zm1.0974-2.3617l2.602-1.4998 2.6069 1.4998v2.9994l-2.5974 1.4997-2.6067-1.4997z"
        fill="#10A37F"
      />
    </svg>
  );
}

/**
 * Google Gemini logo
 */
export function GeminiLogo({ size = 24, className = '' }: LogoProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      <path
        d="M12 24C12 22.4379 11.6928 20.9472 11.1145 19.5711C10.5361 18.195 9.72024 16.9646 8.69664 15.912C7.67304 14.8593 6.46815 14.0199 5.12194 13.4255C3.77573 12.8311 2.31865 12.5161 0.79248 12.5H0V11.5H0.79248C2.31865 11.4839 3.77573 11.1689 5.12194 10.5745C6.46815 9.98011 7.67304 9.14067 8.69664 8.08802C9.72024 7.03538 10.5361 5.80503 11.1145 4.42891C11.6928 3.05279 12 1.56206 12 0H13C13 1.56206 13.3072 3.05279 13.8855 4.42891C14.4639 5.80503 15.2798 7.03538 16.3034 8.08802C17.327 9.14067 18.5319 9.98011 19.8781 10.5745C21.2243 11.1689 22.6814 11.4839 24.2075 11.5H25V12.5H24.2075C22.6814 12.5161 21.2243 12.8311 19.8781 13.4255C18.5319 14.0199 17.327 14.8593 16.3034 15.912C15.2798 16.9646 14.4639 18.195 13.8855 19.5711C13.3072 20.9472 13 22.4379 13 24H12Z"
        fill="url(#gemini_gradient)"
      />
      <defs>
        <linearGradient id="gemini_gradient" x1="0" y1="12" x2="25" y2="12" gradientUnits="userSpaceOnUse">
          <stop stopColor="#4285F4" />
          <stop offset="0.5" stopColor="#9B72CB" />
          <stop offset="1" stopColor="#D96570" />
        </linearGradient>
      </defs>
    </svg>
  );
}

/**
 * Perplexity Sonar logo
 */
export function PerplexityLogo({ size = 24, className = '' }: LogoProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      <path
        fillRule="evenodd"
        d="M8 .188a.5.5 0 0 1 .503.5V4.03l3.022-2.92.059-.048a.51.51 0 0 1 .49-.054.5.5 0 0 1 .306.46v3.247h1.117l.1.01a.5.5 0 0 1 .403.49v5.558a.5.5 0 0 1-.503.5H12.38v3.258a.5.5 0 0 1-.312.462.51.51 0 0 1-.55-.11l-3.016-3.018v3.448c0 .275-.225.5-.503.5a.5.5 0 0 1-.503-.5v-3.448l-3.018 3.019a.51.51 0 0 1-.548.11.5.5 0 0 1-.312-.463v-3.258H2.503a.5.5 0 0 1-.503-.5V5.215l.01-.1c.047-.229.25-.4.493-.4H3.62V1.469l.006-.074a.5.5 0 0 1 .302-.387.51.51 0 0 1 .547.102l3.023 2.92V.687c0-.276.225-.5.503-.5M4.626 9.333v3.984l2.87-2.872v-4.01zm3.877 1.113 2.871 2.871V9.333l-2.87-2.897zm3.733-1.668a.5.5 0 0 1 .145.35v1.145h.612V5.715H9.201zm-9.23 1.495h.613V9.13c0-.131.052-.257.145-.35l3.033-3.064h-3.79zm1.62-5.558H6.76L4.626 2.652zm4.613 0h2.134V2.652z"
        fill="#20B8CD"
      />
    </svg>
  );
}

/**
 * Get the appropriate logo component for a provider
 */
export function AIProviderLogo({
  provider,
  size = 24,
  className = ''
}: {
  provider: 'claude' | 'openai' | 'gemini' | 'perplexity';
  size?: number;
  className?: string;
}) {
  switch (provider) {
    case 'claude':
      return <ClaudeLogo size={size} className={className} />;
    case 'openai':
      return <OpenAILogo size={size} className={className} />;
    case 'gemini':
      return <GeminiLogo size={size} className={className} />;
    case 'perplexity':
      return <PerplexityLogo size={size} className={className} />;
  }
}

/**
 * Provider display names
 */
export const AI_PROVIDER_NAMES = {
  claude: 'Claude',
  openai: 'OpenAI',
  gemini: 'Gemini',
  perplexity: 'Perplexity',
} as const;
