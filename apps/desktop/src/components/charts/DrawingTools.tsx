/**
 * Drawing Tools Component
 * Canvas overlay for drawing trendlines, horizontal lines, and Fibonacci retracements
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
import type { IChartApi } from 'lightweight-charts';

// ============================================================================
// Types
// ============================================================================

export type DrawingTool = 'select' | 'trendline' | 'horizontal' | 'fibonacci';

export interface Point {
  x: number; // pixel
  y: number; // pixel
  time?: string;
  price?: number;
}

export interface Drawing {
  id: string;
  type: DrawingTool;
  points: Point[];
  color: string;
  lineWidth: number;
  // Fibonacci specific
  fibLevels?: number[];
}

export interface DrawingToolsProps {
  chartApi: IChartApi | null;
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

function pointsToPrice(
  chartApi: IChartApi | null,
  point: Point
): { time: string; price: number } | null {
  if (!chartApi) return null;

  try {
    const timeScale = chartApi.timeScale();

    // Convert x to time
    const time = timeScale.coordinateToTime(point.x);
    if (time === null) return null;

    // Note: lightweight-charts v5 doesn't expose direct pixel-to-price conversion
    // We store pixel coordinates and convert time only
    // Price conversion would require access to the series data
    return {
      time: String(time),
      price: point.y, // Store y as pixel coordinate, not actual price
    };
  } catch {
    return null;
  }
}

// ============================================================================
// Drawing Canvas Component
// ============================================================================

export function DrawingTools({
  chartApi,
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

  // ============================================================================
  // Drawing Functions
  // ============================================================================

  const drawLine = useCallback(
    (ctx: CanvasRenderingContext2D, start: Point, end: Point, color: string, lineWidth: number, dashed = false) => {
      ctx.beginPath();
      ctx.strokeStyle = color;
      ctx.lineWidth = lineWidth;
      if (dashed) {
        ctx.setLineDash([5, 5]);
      } else {
        ctx.setLineDash([]);
      }
      ctx.moveTo(start.x, start.y);
      ctx.lineTo(end.x, end.y);
      ctx.stroke();
    },
    []
  );

  const drawHorizontalLine = useCallback(
    (ctx: CanvasRenderingContext2D, y: number, color: string, lineWidth: number) => {
      ctx.beginPath();
      ctx.strokeStyle = color;
      ctx.lineWidth = lineWidth;
      ctx.setLineDash([]);
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();

      // Draw price label
      const priceData = pointsToPrice(chartApi, { x: 0, y });
      if (priceData) {
        ctx.fillStyle = color;
        ctx.font = '11px monospace';
        ctx.fillText(priceData.price.toFixed(2), width - 60, y - 4);
      }
    },
    [width, chartApi]
  );

  const drawFibonacci = useCallback(
    (ctx: CanvasRenderingContext2D, start: Point, end: Point, color: string) => {
      const minY = Math.min(start.y, end.y);
      const maxY = Math.max(start.y, end.y);
      const range = maxY - minY;

      // Draw levels
      FIBONACCI_LEVELS.forEach((level) => {
        const y = maxY - range * level;
        const levelColor = FIBONACCI_COLORS[level] || color;

        ctx.beginPath();
        ctx.strokeStyle = levelColor;
        ctx.lineWidth = 1;
        ctx.setLineDash(level === 0 || level === 1 ? [] : [3, 3]);
        ctx.moveTo(Math.min(start.x, end.x), y);
        ctx.lineTo(Math.max(start.x, end.x), y);
        ctx.stroke();

        // Label
        ctx.fillStyle = levelColor;
        ctx.font = '10px monospace';
        ctx.fillText(`${(level * 100).toFixed(1)}%`, Math.max(start.x, end.x) + 5, y + 3);
      });

      // Draw vertical lines
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
    []
  );

  const drawHandle = useCallback(
    (ctx: CanvasRenderingContext2D, point: Point, isSelected: boolean) => {
      ctx.beginPath();
      ctx.fillStyle = isSelected ? '#2563eb' : '#ffffff';
      ctx.strokeStyle = '#2563eb';
      ctx.lineWidth = 2;
      ctx.arc(point.x, point.y, 5, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    },
    []
  );

  // ============================================================================
  // Render All Drawings
  // ============================================================================

  const renderDrawings = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Draw all saved drawings
    [...drawings, currentDrawing].filter(Boolean).forEach((drawing) => {
      if (!drawing) return;

      const isSelected = drawing.id === selectedDrawingId;
      const isHovered = drawing.id === hoveredDrawingId;
      const color = isSelected || isHovered ? '#3b82f6' : drawing.color;
      const lineWidth = isSelected || isHovered ? drawing.lineWidth + 1 : drawing.lineWidth;

      switch (drawing.type) {
        case 'trendline':
          if (drawing.points.length >= 2) {
            drawLine(ctx, drawing.points[0], drawing.points[1], color, lineWidth);
            if (isSelected) {
              drawHandle(ctx, drawing.points[0], true);
              drawHandle(ctx, drawing.points[1], true);
            }
          } else if (drawing.points.length === 1 && currentDrawing) {
            // Drawing in progress - show start point
            drawHandle(ctx, drawing.points[0], true);
          }
          break;

        case 'horizontal':
          if (drawing.points.length >= 1) {
            drawHorizontalLine(ctx, drawing.points[0].y, color, lineWidth);
            if (isSelected) {
              drawHandle(ctx, { x: 50, y: drawing.points[0].y }, true);
            }
          }
          break;

        case 'fibonacci':
          if (drawing.points.length >= 2) {
            drawFibonacci(ctx, drawing.points[0], drawing.points[1], color);
            if (isSelected) {
              drawHandle(ctx, drawing.points[0], true);
              drawHandle(ctx, drawing.points[1], true);
            }
          } else if (drawing.points.length === 1 && currentDrawing) {
            drawHandle(ctx, drawing.points[0], true);
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
  ]);

  // ============================================================================
  // Mouse Event Handlers
  // ============================================================================

  const getMousePosition = useCallback((e: React.MouseEvent<HTMLCanvasElement>): Point => {
    const canvas = canvasRef.current;
    if (!canvas) return { x: 0, y: 0 };

    const rect = canvas.getBoundingClientRect();
    return {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top,
    };
  }, []);

  const findDrawingAtPoint = useCallback(
    (point: Point): Drawing | null => {
      // Check drawings in reverse order (top to bottom)
      for (let i = drawings.length - 1; i >= 0; i--) {
        const drawing = drawings[i];

        switch (drawing.type) {
          case 'trendline':
            if (drawing.points.length >= 2) {
              const [p1, p2] = drawing.points;
              // Check if point is near the line
              const dist = pointToLineDistance(point, p1, p2);
              if (dist < 10) return drawing;
            }
            break;

          case 'horizontal':
            if (drawing.points.length >= 1) {
              if (Math.abs(point.y - drawing.points[0].y) < 10) return drawing;
            }
            break;

          case 'fibonacci':
            if (drawing.points.length >= 2) {
              const [p1, p2] = drawing.points;
              const minX = Math.min(p1.x, p2.x);
              const maxX = Math.max(p1.x, p2.x);
              const minY = Math.min(p1.y, p2.y);
              const maxY = Math.max(p1.y, p2.y);
              if (point.x >= minX - 10 && point.x <= maxX + 10 && point.y >= minY - 10 && point.y <= maxY + 10) {
                return drawing;
              }
            }
            break;
        }
      }
      return null;
    },
    [drawings]
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
        setDrawings((prev) => [...prev, newDrawing]);
        onDrawingsChange?.([...drawings, newDrawing]);
        setActiveTool('select');
      } else {
        setCurrentDrawing(newDrawing);
        setIsDrawing(true);
      }
    },
    [enabled, activeTool, getMousePosition, findDrawingAtPoint, drawings, onDrawingsChange]
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      if (!enabled) return;

      const point = getMousePosition(e);

      if (isDrawing && currentDrawing) {
        // Update current drawing's second point
        setCurrentDrawing({
          ...currentDrawing,
          points: [currentDrawing.points[0], point],
        });
      } else if (activeTool === 'select') {
        // Check for hover
        const drawing = findDrawingAtPoint(point);
        setHoveredDrawingId(drawing?.id || null);
      }
    },
    [enabled, isDrawing, currentDrawing, activeTool, getMousePosition, findDrawingAtPoint]
  );

  const handleMouseUp = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      if (!enabled || !isDrawing || !currentDrawing) return;

      const point = getMousePosition(e);

      // Complete the drawing
      const completedDrawing: Drawing = {
        ...currentDrawing,
        points: [currentDrawing.points[0], point],
      };

      setDrawings((prev) => [...prev, completedDrawing]);
      onDrawingsChange?.([...drawings, completedDrawing]);
      setCurrentDrawing(null);
      setIsDrawing(false);
      setActiveTool('select');
    },
    [enabled, isDrawing, currentDrawing, getMousePosition, drawings, onDrawingsChange]
  );

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Delete' || e.key === 'Backspace') {
        if (selectedDrawingId) {
          const newDrawings = drawings.filter((d) => d.id !== selectedDrawingId);
          setDrawings(newDrawings);
          onDrawingsChange?.(newDrawings);
          setSelectedDrawingId(null);
        }
      } else if (e.key === 'Escape') {
        setCurrentDrawing(null);
        setIsDrawing(false);
        setActiveTool('select');
      }
    },
    [selectedDrawingId, drawings, onDrawingsChange]
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
      const newDrawings = drawings.filter((d) => d.id !== selectedDrawingId);
      setDrawings(newDrawings);
      onDrawingsChange?.(newDrawings);
      setSelectedDrawingId(null);
    }
  }, [selectedDrawingId, drawings, onDrawingsChange]);

  if (!enabled) return null;

  return (
    <div className="absolute inset-0 pointer-events-none">
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

function pointToLineDistance(point: Point, lineStart: Point, lineEnd: Point): number {
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
