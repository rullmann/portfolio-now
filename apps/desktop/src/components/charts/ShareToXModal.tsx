import { useState, useEffect, useCallback, useRef } from 'react';
import { X, Upload, CheckCircle, AlertCircle, Loader2, ExternalLink } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useEscapeKey } from '../../lib/hooks';
import { useSettingsStore, toast } from '../../store';
import { useTwitterAuth } from '../../hooks/useTwitterAuth';
import { composeTweet, splitIntoThreadChunks } from '../../lib/tweetComposer';
import { captureChartForTweet } from '../../lib/tweetImage';
import type { TrendInfo, RiskRewardAnalysis } from '../../lib/types';
import type { TechnicalSignal } from '../../lib/signals';
import type { ShareSecurityInfo } from './ShareToXButton';

const TWEET_MAX_LENGTH = 280;

type PostState = 'idle' | 'capturing' | 'uploading' | 'posting' | 'success' | 'error';

interface ShareToXModalProps {
  chartRef: React.RefObject<HTMLDivElement>;
  security: ShareSecurityInfo;
  currentPrice: number;
  analysis?: string;
  trendInfo?: TrendInfo;
  riskReward?: RiskRewardAnalysis;
  signals?: TechnicalSignal[];
  patterns?: string[];
  onClose: () => void;
}

export function ShareToXModal({
  chartRef,
  security,
  currentPrice,
  analysis,
  trendInfo,
  riskReward,
  signals,
  patterns,
  onClose,
}: ShareToXModalProps) {
  const { isConnected, isConnecting, username, connect, getAccessToken } = useTwitterAuth();
  const { language, twitterDefaultHashtags, twitterIncludeThread, twitterIncludeWatermark } = useSettingsStore();

  const [tweetText, setTweetText] = useState('');
  const [includeThread, setIncludeThread] = useState(twitterIncludeThread);
  const [previewImage, setPreviewImage] = useState<string | null>(null);
  const [imageBase64, setImageBase64] = useState<string | null>(null);
  const [postState, setPostState] = useState<PostState>('idle');
  const [tweetUrl, setTweetUrl] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const textAreaRef = useRef<HTMLTextAreaElement>(null);

  useEscapeKey(true, onClose);

  // Generate initial tweet text
  useEffect(() => {
    const text = composeTweet({
      securityName: security.name,
      ticker: security.ticker,
      currentPrice,
      currency: security.currency,
      trendInfo,
      riskReward,
      signals,
      patterns,
      hashtags: twitterDefaultHashtags,
      language,
    });
    setTweetText(text);
  }, [security, currentPrice, trendInfo, riskReward, signals, patterns, twitterDefaultHashtags, language]);

  // Capture chart preview
  useEffect(() => {
    if (!chartRef.current) return;
    try {
      const result = captureChartForTweet(chartRef.current, {
        securityName: security.name,
        ticker: security.ticker,
        currentPrice,
        currency: security.currency,
        trendInfo,
        riskReward,
        signals,
        includeWatermark: twitterIncludeWatermark,
      });
      setPreviewImage(`data:image/jpeg;base64,${result.base64}`);
      setImageBase64(result.base64);
    } catch (error) {
      console.error('Failed to capture chart:', error);
    }
  }, [chartRef, security, currentPrice, trendInfo, riskReward, signals, twitterIncludeWatermark]);

  const charCount = tweetText.length;
  const isOverLimit = charCount > TWEET_MAX_LENGTH;

  const handleConnect = useCallback(async () => {
    try {
      await connect();
      toast.success('Mit X verbunden!');
    } catch (error) {
      toast.error(String(error));
    }
  }, [connect]);

  const handlePost = useCallback(async () => {
    if (!imageBase64 || isOverLimit) return;

    try {
      setPostState('uploading');
      setErrorMessage(null);

      const accessToken = await getAccessToken();
      if (!accessToken) {
        setErrorMessage('Nicht mit X verbunden. Bitte zuerst verbinden.');
        setPostState('error');
        return;
      }

      // Upload media
      const uploadResult = await invoke<{ media_id_string: string }>('twitter_upload_media', {
        accessToken,
        imageBase64,
      });

      // Post main tweet
      setPostState('posting');
      const tweetResult = await invoke<{ tweet_id: string; tweet_url: string }>('twitter_post_tweet', {
        accessToken,
        text: tweetText,
        mediaIds: [uploadResult.media_id_string],
        replyToTweetId: null,
      });

      // Post thread (reply with full analysis) if enabled
      if (includeThread && analysis) {
        const chunks = splitIntoThreadChunks(analysis);
        let lastTweetId = tweetResult.tweet_id;

        for (const chunk of chunks) {
          const replyResult = await invoke<{ tweet_id: string; tweet_url: string }>('twitter_post_tweet', {
            accessToken,
            text: chunk,
            mediaIds: null,
            replyToTweetId: lastTweetId,
          });
          lastTweetId = replyResult.tweet_id;
        }
      }

      setTweetUrl(tweetResult.tweet_url);
      setPostState('success');
      toast.success('Tweet gepostet!');
    } catch (error) {
      console.error('Failed to post tweet:', error);
      setErrorMessage(String(error));
      setPostState('error');
      toast.error('Tweet konnte nicht gepostet werden.');
    }
  }, [imageBase64, isOverLimit, getAccessToken, tweetText, includeThread, analysis]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />

      {/* Modal */}
      <div
        role="dialog"
        aria-modal="true"
        className="relative bg-card border border-border rounded-lg shadow-lg w-full max-w-lg mx-4 overflow-hidden"
      >
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <h2 className="text-lg font-semibold">Auf X teilen</h2>
          <button
            onClick={onClose}
            aria-label="Schließen"
            className="p-1 hover:bg-muted rounded-md transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="p-4 space-y-4 max-h-[70vh] overflow-y-auto">
          {/* Chart Preview */}
          {previewImage && (
            <div className="rounded-lg overflow-hidden border border-border">
              <img
                src={previewImage}
                alt="Chart-Vorschau"
                className="w-full h-auto"
              />
            </div>
          )}

          {/* Tweet Text */}
          <div className="space-y-1">
            <textarea
              ref={textAreaRef}
              value={tweetText}
              onChange={(e) => setTweetText(e.target.value)}
              rows={5}
              disabled={postState === 'success'}
              className="w-full px-3 py-2 bg-background border border-border rounded-lg text-sm resize-none focus:outline-none focus:ring-2 focus:ring-primary/50 disabled:opacity-50 disabled:cursor-not-allowed"
            />
            <div className="flex justify-end">
              <span className={`text-xs ${isOverLimit ? 'text-red-500 font-semibold' : 'text-muted-foreground'}`}>
                {charCount}/{TWEET_MAX_LENGTH} {!isOverLimit && '\u2713'}
              </span>
            </div>
          </div>

          {/* Thread Option */}
          {analysis && (
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={includeThread}
                onChange={(e) => setIncludeThread(e.target.checked)}
                disabled={postState === 'success'}
                className="rounded border-border"
              />
              <span className="text-sm">Als Thread posten (volle Analyse)</span>
            </label>
          )}

          {/* Status Messages */}
          {postState === 'success' && tweetUrl && (
            <div className="flex items-center gap-2 p-3 bg-green-500/10 border border-green-500/30 rounded-lg">
              <CheckCircle size={16} className="text-green-500" />
              <span className="text-sm text-green-600 dark:text-green-400">Tweet gepostet!</span>
              <a
                href={tweetUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="ml-auto flex items-center gap-1 text-sm text-primary hover:underline"
                onClick={(e) => {
                  e.preventDefault();
                  import('@tauri-apps/plugin-shell').then(({ open }) => open(tweetUrl));
                }}
              >
                Ansehen <ExternalLink size={12} />
              </a>
            </div>
          )}

          {postState === 'error' && errorMessage && (
            <div className="flex items-center gap-2 p-3 bg-red-500/10 border border-red-500/30 rounded-lg">
              <AlertCircle size={16} className="text-red-500" />
              <span className="text-sm text-red-600 dark:text-red-400">{errorMessage}</span>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-border flex items-center justify-between">
          {isConnected ? (
            <span className="text-xs text-muted-foreground">
              Verbunden als @{username}
            </span>
          ) : (
            <span className="text-xs text-muted-foreground">
              Nicht verbunden
            </span>
          )}

          <div className="flex items-center gap-2">
            {!isConnected ? (
              <button
                onClick={handleConnect}
                disabled={isConnecting}
                className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isConnecting ? (
                  <>
                    <Loader2 size={14} className="animate-spin" />
                    Verbinde...
                  </>
                ) : (
                  'Mit X verbinden'
                )}
              </button>
            ) : postState === 'success' ? (
              <button
                onClick={onClose}
                className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
              >
                Schließen
              </button>
            ) : (
              <button
                onClick={handlePost}
                disabled={isOverLimit || !imageBase64 || postState === 'uploading' || postState === 'posting'}
                className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {postState === 'uploading' || postState === 'posting' ? (
                  <>
                    <Loader2 size={14} className="animate-spin" />
                    {postState === 'uploading' ? 'Bild hochladen...' : 'Posten...'}
                  </>
                ) : (
                  <>
                    <Upload size={14} />
                    Posten
                  </>
                )}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
