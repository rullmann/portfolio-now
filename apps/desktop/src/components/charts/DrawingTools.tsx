/**
 * Drawing Tools Component
 * Canvas overlay for drawing trendlines, horizontal lines, and Fibonacci retracements
 * Uses price/time coordinates so drawings stay in place when zooming/scrolling
 */

import { useRef, useEffect, useState, useCallback } from 'react';
import {
  Minus,
  TrendingUp,
  Hash,
  Trash2,
  MousePointer,
  X,
} from 'lucide-react';
import type { IChartApi, ISeriesApi } from 'lightweight-charts';

// ============================================================================
// Types
// ============================================================================

export type DrawingTool = 'select' | 'trendline' | 'horizontal' | 'fibonacci';

export interface Point {
  x: number; // pixel (transient, for rendering)
  y: number; // pixel (transient, for rendering)
  time?: string;
  price?: number;
}

export interface Drawing {
  id: string;
  type: DrawingTool;
  points: Point[];
  color: string;
  lineWidth: number;
  fibLevels?: number[];
}

export interface DrawingToolsProps {
  chartApi: IChartApi | null;
  mainSeries: ISeriesApi<'Candlestick'> | null;
  width: number;
  height: number;
  enabled: boolean;
  onDrawingsChange?: (drawings: Drawing[]) => void;
  initialDrawings?: Drawing[];
}

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_COLOR = '#2563eb';
const FIBONACCI_LEVELS = [0, 0.236, 0.382, 0.5, 0.618, 0.786, 1];
const FIBONACCI_COLORS: Record<number, string> = {
  0: '#ef4444',
  0.236: '#f97316',
  0.382: '#eab308',
  0.5: '#22c55e',
  0.618: '#06b6d4',
  0.786: '#8b5cf6',
  1: '#ef4444',
};

// ============================================================================
// Helper Functions
// ============================================================================

function generateId(): string {
  return `drawing-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

/** Convert pixel coordinates to price/time using chart API */
function pixelToLogical(
  chartApi: IChartApi | null,
  mainSeries: ISeriesApi<'Candlestick'> | null,
  px: number,
  py: number,
): { time: string; price: number } | null {
  if (!chartApi || !mainSeries) return null;
  try {
    const timeScale = chartApi.timeScale();
    const time = timeScale.coordinateToTime(px);
    if (time === null || time === undefined) return null;
    const price = mainSeries.coordinateToPrice(py);
    if (price === null || price === undefined) return null;
    return { time: String(time), price: price as number };
  } catch {
    return null;
  }
}

/** Convert price/time back to pixel coordinates */
function logicalToPixel(
  chartApi: IChartApi | null,
  mainSeries: ISeriesApi<'Candlestick'> | null,
  time: string | undefined,
  price: number | undefined,
): { x: number; y: number } | null {
  if (!chartApi || !mainSeries || !time || price === undefined) return null;
  try {
    const timeScale = chartApi.timeScale();
    const x = timeScale.timeToCoordinate(time as unknown as import('lightweight-charts').Time);
    if (x === null || x === undefined) return null;
    const y = mainSeries.priceToCoordinate(price);
    if (y === null || y === undefined) return null;
    return { x: x as number, y: y as number };
  } catch {
    return null;
  }
}

// ============================================================================
// Drawing Canvas Component
// ============================================================================

export function DrawingTools({
  chartApi,
  mainSeries,
  width,
  height,
  enabled,
  onDrawingsChange,
  initialDrawings = [],
}: DrawingToolsProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [activeTool, setActiveTool] = useState<DrawingTool>('select');
  const [drawings, setDrawings] = useState<Drawing[]>(initialDrawings);
  const [currentDrawing, setCurrentDrawing] = useState<Drawing | null>(null);
  const [isDrawing, setIsDrawing] = useState(false);
  const [selectedDrawingId, setSelectedDrawingId] = useState<string | null>(null);
  const [hoveredDrawingId, setHoveredDrawingId] = useState<string | null>(null);

  // Sync initialDrawings when they change (e.g. security switch)
  useEffect(() => {
    setDrawings(initialDrawings);
  }, [initialDrawings]);

  // ============================================================================
  // Coordinate Conversion Helpers
  // ============================================================================

  /** Convert a stored point (with time/price) to current pixel position */
  const toPixel = useCallback(
    (point: Point): { x: number; y: number } | null => {
      if (point.time && point.price !== undefined) {
        return logicalToPixel(chartApi, mainSeries, point.time, point.price);
      }
      // Fallback: use raw pixel (for in-progress drawing before saving)
      return { x: point.x, y: point.y };
    },
    [chartApi, mainSeries],
  );

  // ============================================================================
  // Drawing Functions
  // ============================================================================

  const drawLine = useCallback(
    (ctx: CanvasRenderingContext2D, start: { x: number; y: number }, end: { x: number; y: number }, color: string, lineWidth: number, dashed = false) => {
      ctx.beginPath();
      ctx.strokeStyle = color;
      ctx.lineWidth = lineWidth;
      ctx.setLineDash(dashed ? [5, 5] : []);
      ctx.moveTo(start.x, start.y);
      ctx.lineTo(end.x, end.y);
      ctx.stroke();
    },
    [],
  );

  const drawHorizontalLine = useCallback(
    (ctx: CanvasRenderingContext2D, y: number, price: number | undefined, color: string, lineWidth: number) => {
      ctx.beginPath();
      ctx.strokeStyle = color;
      ctx.lineWidth = lineWidth;
      ctx.setLineDash([]);
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();

      // Draw price label
      if (price !== undefined) {
        ctx.fillStyle = color;
        ctx.font = '11px monospace';
        ctx.fillText(price.toFixed(2), width - 70, y - 4);
      }
    },
    [width],
  );

  const drawFibonacci = useCallback(
    (ctx: CanvasRenderingContext2D, start: { x: number; y: number }, end: { x: number; y: number }, startPrice: number, endPrice: number, color: string) => {
      const minY = Math.min(start.y, end.y);
      const maxY = Math.max(start.y, end.y);
      const range = maxY - minY;
      const minPrice = Math.min(startPrice, endPrice);
      const priceRange = Math.abs(endPrice - startPrice);

      FIBONACCI_LEVELS.forEach((level) => {
        const y = maxY - range * level;
        const levelColor = FIBONACCI_COLORS[level] || color;
        const levelPrice = minPrice + priceRange * level;

        ctx.beginPath();
        ctx.strokeStyle = levelColor;
        ctx.lineWidth = 1;
        ctx.setLineDash(level === 0 || level === 1 ? [] : [3, 3]);
        ctx.moveTo(Math.min(start.x, end.x), y);
        ctx.lineTo(Math.max(start.x, end.x), y);
        ctx.stroke();

        // Label with level % and price
        ctx.fillStyle = levelColor;
        ctx.font = '10px monospace';
        ctx.fillText(`${(level * 100).toFixed(1)}% (${levelPrice.toFixed(2)})`, Math.max(start.x, end.x) + 5, y + 3);
      });

      // Vertical boundary lines
      ctx.beginPath();
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.setLineDash([2, 2]);
      ctx.moveTo(start.x, minY);
      ctx.lineTo(start.x, maxY);
      ctx.moveTo(end.x, minY);
      ctx.lineTo(end.x, maxY);
      ctx.stroke();
    },
    [],
  );

  const drawHandle = useCallback(
    (ctx: CanvasRenderingContext2D, point: { x: number; y: number }, isSelected: boolean) => {
      ctx.beginPath();
      ctx.fillStyle = isSelected ? '#2563eb' : '#ffffff';
      ctx.strokeStyle = '#2563eb';
      ctx.lineWidth = 2;
      ctx.setLineDash([]);
      ctx.arc(point.x, point.y, 5, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    },
    [],
  );

  // ============================================================================
  // Render All Drawings
  // ============================================================================

  const renderDrawings = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    ctx.clearRect(0, 0, width, height);

    [...drawings, currentDrawing].filter(Boolean).forEach((drawing) => {
      if (!drawing) return;

      const isSelected = drawing.id === selectedDrawingId;
      const isHovered = drawing.id === hoveredDrawingId;
      const color = isSelected || isHovered ? '#3b82f6' : drawing.color;
      const lw = isSelected || isHovered ? drawing.lineWidth + 1 : drawing.lineWidth;

      // Convert stored points to pixels
      const pixelPoints = drawing.points
        .map((p) => toPixel(p))
        .filter((p): p is { x: number; y: number } => p !== null);

      switch (drawing.type) {
        case 'trendline':
          if (pixelPoints.length >= 2) {
            drawLine(ctx, pixelPoints[0], pixelPoints[1], color, lw);
            if (isSelected) {
              drawHandle(ctx, pixelPoints[0], true);
              drawHandle(ctx, pixelPoints[1], true);
            }
          } else if (pixelPoints.length === 1 && currentDrawing) {
            drawHandle(ctx, pixelPoints[0], true);
          }
          break;

        case 'horizontal':
          if (pixelPoints.length >= 1) {
            drawHorizontalLine(ctx, pixelPoints[0].y, drawing.points[0].price, color, lw);
            if (isSelected) {
              drawHandle(ctx, { x: 50, y: pixelPoints[0].y }, true);
            }
          }
          break;

        case 'fibonacci':
          if (pixelPoints.length >= 2) {
            const p0Price = drawing.points[0].price ?? 0;
            const p1Price = drawing.points[1].price ?? 0;
            drawFibonacci(ctx, pixelPoints[0], pixelPoints[1], p0Price, p1Price, color);
            if (isSelected) {
              drawHandle(ctx, pixelPoints[0], true);
              drawHandle(ctx, pixelPoints[1], true);
            }
          } else if (pixelPoints.length === 1 && currentDrawing) {
            drawHandle(ctx, pixelPoints[0], true);
          }
          break;
      }
    });
  }, [
    drawings,
    currentDrawing,
    selectedDrawingId,
    hoveredDrawingId,
    width,
    height,
    drawLine,
    drawHorizontalLine,
    drawFibonacci,
    drawHandle,
    toPixel,
  ]);

  // ============================================================================
  // Mouse Event Handlers
  // ============================================================================

  const getMousePosition = useCallback((e: React.MouseEvent<HTMLCanvasElement>): Point => {
    const canvas = canvasRef.current;
    if (!canvas) return { x: 0, y: 0 };

    const rect = canvas.getBoundingClientRect();
    const px = e.clientX - rect.left;
    const py = e.clientY - rect.top;

    // Enrich with logical coordinates
    const logical = pixelToLogical(chartApi, mainSeries, px, py);
    return {
      x: px,
      y: py,
      time: logical?.time,
      price: logical?.price,
    };
  }, [chartApi, mainSeries]);

  const findDrawingAtPoint = useCallback(
    (point: { x: number; y: number }): Drawing | null => {
      for (let i = drawings.length - 1; i >= 0; i--) {
        const drawing = drawings[i];
        const pixelPoints = drawing.points
          .map((p) => toPixel(p))
          .filter((p): p is { x: number; y: number } => p !== null);

        switch (drawing.type) {
          case 'trendline':
            if (pixelPoints.length >= 2) {
              const dist = pointToLineDistance(point, pixelPoints[0], pixelPoints[1]);
              if (dist < 10) return drawing;
            }
            break;

          case 'horizontal':
            if (pixelPoints.length >= 1) {
              if (Math.abs(point.y - pixelPoints[0].y) < 10) return drawing;
            }
            break;

          case 'fibonacci':
            if (pixelPoints.length >= 2) {
              const minX = Math.min(pixelPoints[0].x, pixelPoints[1].x);
              const maxX = Math.max(pixelPoints[0].x, pixelPoints[1].x);
              const minY = Math.min(pixelPoints[0].y, pixelPoints[1].y);
              const maxY = Math.max(pixelPoints[0].y, pixelPoints[1].y);
              if (point.x >= minX - 10 && point.x <= maxX + 10 && point.y >= minY - 10 && point.y <= maxY + 10) {
                return drawing;
              }
            }
            break;
        }
      }
      return null;
    },
    [drawings, toPixel],
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      if (!enabled) return;

      const point = getMousePosition(e);

      if (activeTool === 'select') {
        const drawing = findDrawingAtPoint(point);
        setSelectedDrawingId(drawing?.id || null);
        return;
      }

      // Start new drawing
      const newDrawing: Drawing = {
        id: generateId(),
        type: activeTool,
        points: [point],
        color: DEFAULT_COLOR,
        lineWidth: 2,
      };

      if (activeTool === 'horizontal') {
        // Horizontal line only needs one point
        const completed = { ...newDrawing };
        setDrawings((prev) => {
          const next = [...prev, completed];
          onDrawingsChange?.(next);
          return next;
        });
        setActiveTool('select');
      } else {
        setCurrentDrawing(newDrawing);
        setIsDrawing(true);
      }
    },
    [enabled, activeTool, getMousePosition, findDrawingAtPoint, onDrawingsChange],
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      if (!enabled) return;

      const point = getMousePosition(e);

      if (isDrawing && currentDrawing) {
        setCurrentDrawing({
          ...currentDrawing,
          points: [currentDrawing.points[0], point],
        });
      } else if (activeTool === 'select') {
        const drawing = findDrawingAtPoint(point);
        setHoveredDrawingId(drawing?.id || null);
      }
    },
    [enabled, isDrawing, currentDrawing, activeTool, getMousePosition, findDrawingAtPoint],
  );

  const handleMouseUp = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      if (!enabled || !isDrawing || !currentDrawing) return;

      const point = getMousePosition(e);

      const completedDrawing: Drawing = {
        ...currentDrawing,
        points: [currentDrawing.points[0], point],
      };

      setDrawings((prev) => {
        const next = [...prev, completedDrawing];
        onDrawingsChange?.(next);
        return next;
      });
      setCurrentDrawing(null);
      setIsDrawing(false);
      setActiveTool('select');
    },
    [enabled, isDrawing, currentDrawing, getMousePosition, onDrawingsChange],
  );

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Delete' || e.key === 'Backspace') {
        if (selectedDrawingId) {
          setDrawings((prev) => {
            const next = prev.filter((d) => d.id !== selectedDrawingId);
            onDrawingsChange?.(next);
            return next;
          });
          setSelectedDrawingId(null);
        }
      } else if (e.key === 'Escape') {
        setCurrentDrawing(null);
        setIsDrawing(false);
        setActiveTool('select');
      }
    },
    [selectedDrawingId, onDrawingsChange],
  );

  // ============================================================================
  // Effects
  // ============================================================================

  useEffect(() => {
    renderDrawings();
  }, [renderDrawings]);

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  // Update canvas size
  useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas) {
      canvas.width = width;
      canvas.height = height;
      renderDrawings();
    }
  }, [width, height, renderDrawings]);

  // Re-render drawings when chart view changes (scroll/zoom)
  useEffect(() => {
    if (!chartApi || !enabled) return;

    const timeScale = chartApi.timeScale();
    const handler = () => renderDrawings();
    timeScale.subscribeVisibleTimeRangeChange(handler);
    return () => {
      try { timeScale.unsubscribeVisibleTimeRangeChange(handler); } catch { /* chart may be disposed */ }
    };
  }, [chartApi, enabled, renderDrawings]);

  // ============================================================================
  // Actions
  // ============================================================================

  const clearAllDrawings = useCallback(() => {
    setDrawings([]);
    setSelectedDrawingId(null);
    onDrawingsChange?.([]);
  }, [onDrawingsChange]);

  const deleteSelected = useCallback(() => {
    if (selectedDrawingId) {
      setDrawings((prev) => {
        const next = prev.filter((d) => d.id !== selectedDrawingId);
        onDrawingsChange?.(next);
        return next;
      });
      setSelectedDrawingId(null);
    }
  }, [selectedDrawingId, onDrawingsChange]);

  if (!enabled) return null;

  return (
    <div className="absolute inset-0 pointer-events-none" style={{ zIndex: 20 }}>
      {/* Toolbar */}
      <div className="absolute top-2 left-2 flex gap-1 bg-card/90 backdrop-blur-sm border border-border rounded-lg p-1 pointer-events-auto z-10">
        <button
          onClick={() => setActiveTool('select')}
          className={`p-1.5 rounded transition-colors ${
            activeTool === 'select' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'
          }`}
          title="Auswählen (Esc)"
        >
          <MousePointer size={16} />
        </button>
        <button
          onClick={() => setActiveTool('trendline')}
          className={`p-1.5 rounded transition-colors ${
            activeTool === 'trendline' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'
          }`}
          title="Trendlinie"
        >
          <TrendingUp size={16} />
        </button>
        <button
          onClick={() => setActiveTool('horizontal')}
          className={`p-1.5 rounded transition-colors ${
            activeTool === 'horizontal' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'
          }`}
          title="Horizontale Linie"
        >
          <Minus size={16} />
        </button>
        <button
          onClick={() => setActiveTool('fibonacci')}
          className={`p-1.5 rounded transition-colors ${
            activeTool === 'fibonacci' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'
          }`}
          title="Fibonacci Retracement"
        >
          <Hash size={16} />
        </button>
        <div className="w-px bg-border mx-1" />
        {selectedDrawingId && (
          <button
            onClick={deleteSelected}
            className="p-1.5 rounded hover:bg-red-500/20 text-red-500 transition-colors"
            title="Löschen (Entf)"
          >
            <X size={16} />
          </button>
        )}
        <button
          onClick={clearAllDrawings}
          className="p-1.5 rounded hover:bg-muted transition-colors text-muted-foreground"
          title="Alle löschen"
        >
          <Trash2 size={16} />
        </button>
      </div>

      {/* Drawing Canvas */}
      <canvas
        ref={canvasRef}
        width={width}
        height={height}
        className="absolute inset-0 pointer-events-auto"
        style={{ cursor: activeTool === 'select' ? 'default' : 'crosshair' }}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={() => {
          if (isDrawing) {
            setCurrentDrawing(null);
            setIsDrawing(false);
          }
        }}
      />
    </div>
  );
}

// ============================================================================
// Utility Functions
// ============================================================================

function pointToLineDistance(point: { x: number; y: number }, lineStart: { x: number; y: number }, lineEnd: { x: number; y: number }): number {
  const A = point.x - lineStart.x;
  const B = point.y - lineStart.y;
  const C = lineEnd.x - lineStart.x;
  const D = lineEnd.y - lineStart.y;

  const dot = A * C + B * D;
  const lenSq = C * C + D * D;
  let param = -1;

  if (lenSq !== 0) {
    param = dot / lenSq;
  }

  let xx, yy;

  if (param < 0) {
    xx = lineStart.x;
    yy = lineStart.y;
  } else if (param > 1) {
    xx = lineEnd.x;
    yy = lineEnd.y;
  } else {
    xx = lineStart.x + param * C;
    yy = lineStart.y + param * D;
  }

  const dx = point.x - xx;
  const dy = point.y - yy;

  return Math.sqrt(dx * dx + dy * dy);
}

export default DrawingTools;
