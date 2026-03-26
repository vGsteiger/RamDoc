<script lang="ts">
  import { onMount } from 'svelte';
  import type { OutcomeScore } from '$lib/api';

  interface Props {
    scores: OutcomeScore[];
    scaleType: string;
  }

  let { scores, scaleType }: Props = $props();
  let canvas: HTMLCanvasElement;
  let hoveredPoint: { score: OutcomeScore; x: number; y: number } | null = $state(null);

  // Define severity bands for different scales
  const severityBands = {
    'PHQ-9': [
      { max: 4, label: 'Minimal', color: 'rgba(34, 197, 94, 0.1)' },
      { max: 9, label: 'Mild', color: 'rgba(250, 204, 21, 0.1)' },
      { max: 14, label: 'Moderate', color: 'rgba(249, 115, 22, 0.1)' },
      { max: 19, label: 'Moderately Severe', color: 'rgba(239, 68, 68, 0.1)' },
      { max: 27, label: 'Severe', color: 'rgba(220, 38, 38, 0.15)' },
    ],
    'GAD-7': [
      { max: 4, label: 'Minimal', color: 'rgba(34, 197, 94, 0.1)' },
      { max: 9, label: 'Mild', color: 'rgba(250, 204, 21, 0.1)' },
      { max: 14, label: 'Moderate', color: 'rgba(249, 115, 22, 0.1)' },
      { max: 21, label: 'Severe', color: 'rgba(239, 68, 68, 0.15)' },
    ],
    'BDI-II': [
      { max: 13, label: 'Minimal', color: 'rgba(34, 197, 94, 0.1)' },
      { max: 19, label: 'Mild', color: 'rgba(250, 204, 21, 0.1)' },
      { max: 28, label: 'Moderate', color: 'rgba(249, 115, 22, 0.1)' },
      { max: 63, label: 'Severe', color: 'rgba(239, 68, 68, 0.15)' },
    ],
  };

  const getMaxScore = (scale: string): number => {
    if (scale === 'PHQ-9') return 27;
    if (scale === 'GAD-7') return 21;
    if (scale === 'BDI-II') return 63;
    return 30;
  };

  const formatDate = (dateStr: string): string => {
    const date = new Date(dateStr);
    return date.toLocaleDateString('de-CH', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  };

  const drawChart = () => {
    if (!canvas || scores.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;
    const padding = { top: 30, right: 80, bottom: 60, left: 50 };
    const chartWidth = width - padding.left - padding.right;
    const chartHeight = height - padding.top - padding.bottom;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Sort scores by date
    const sortedScores = [...scores].sort(
      (a, b) => new Date(a.administered_at).getTime() - new Date(b.administered_at).getTime()
    );

    const maxScore = getMaxScore(scaleType);
    const bands = severityBands[scaleType as keyof typeof severityBands] || [];

    // Draw severity bands
    bands.forEach((band, index) => {
      const prevMax = index > 0 ? bands[index - 1].max : 0;
      const yStart = padding.top + chartHeight * (1 - band.max / maxScore);
      const yEnd = padding.top + chartHeight * (1 - prevMax / maxScore);

      ctx.fillStyle = band.color;
      ctx.fillRect(padding.left, yStart, chartWidth, yEnd - yStart);

      // Draw band label
      ctx.fillStyle = '#6B7280';
      ctx.font = '11px sans-serif';
      ctx.textAlign = 'left';
      ctx.fillText(band.label, padding.left + chartWidth + 5, (yStart + yEnd) / 2 + 4);
    });

    // Draw grid lines
    ctx.strokeStyle = '#E5E7EB';
    ctx.lineWidth = 1;

    // Horizontal grid lines
    for (let i = 0; i <= 5; i++) {
      const y = padding.top + (chartHeight / 5) * i;
      ctx.beginPath();
      ctx.moveTo(padding.left, y);
      ctx.lineTo(padding.left + chartWidth, y);
      ctx.stroke();

      // Y-axis labels
      const score = Math.round(maxScore * (1 - i / 5));
      ctx.fillStyle = '#6B7280';
      ctx.font = '12px sans-serif';
      ctx.textAlign = 'right';
      ctx.fillText(score.toString(), padding.left - 10, y + 4);
    }

    if (sortedScores.length === 0) return;

    // Draw data line and points
    ctx.strokeStyle = '#3B82F6';
    ctx.lineWidth = 2;
    ctx.beginPath();

    sortedScores.forEach((score, index) => {
      const x = padding.left + (chartWidth / Math.max(sortedScores.length - 1, 1)) * index;
      const y = padding.top + chartHeight * (1 - score.score / maxScore);

      if (index === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    });
    ctx.stroke();

    // Draw points
    sortedScores.forEach((score, index) => {
      const x = padding.left + (chartWidth / Math.max(sortedScores.length - 1, 1)) * index;
      const y = padding.top + chartHeight * (1 - score.score / maxScore);

      ctx.fillStyle = '#3B82F6';
      ctx.beginPath();
      ctx.arc(x, y, 5, 0, 2 * Math.PI);
      ctx.fill();

      // Draw date label
      ctx.fillStyle = '#6B7280';
      ctx.font = '11px sans-serif';
      ctx.textAlign = 'center';
      ctx.save();
      ctx.translate(x, padding.top + chartHeight + 15);
      ctx.rotate(-Math.PI / 4);
      ctx.fillText(formatDate(score.administered_at), 0, 0);
      ctx.restore();
    });

    // Draw axes
    ctx.strokeStyle = '#6B7280';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(padding.left, padding.top);
    ctx.lineTo(padding.left, padding.top + chartHeight);
    ctx.lineTo(padding.left + chartWidth, padding.top + chartHeight);
    ctx.stroke();

    // Y-axis label
    ctx.fillStyle = '#374151';
    ctx.font = 'bold 13px sans-serif';
    ctx.textAlign = 'center';
    ctx.save();
    ctx.translate(15, padding.top + chartHeight / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText('Score', 0, 0);
    ctx.restore();

    // X-axis label
    ctx.textAlign = 'center';
    ctx.fillText('Date', padding.left + chartWidth / 2, height - 10);
  };

  const handleMouseMove = (event: MouseEvent) => {
    if (!canvas || scores.length === 0) return;

    const rect = canvas.getBoundingClientRect();
    const mouseX = event.clientX - rect.left;
    const mouseY = event.clientY - rect.top;

    const width = canvas.width;
    const height = canvas.height;
    const padding = { top: 30, right: 80, bottom: 60, left: 50 };
    const chartWidth = width - padding.left - padding.right;
    const chartHeight = height - padding.top - padding.bottom;
    const maxScore = getMaxScore(scaleType);

    const sortedScores = [...scores].sort(
      (a, b) => new Date(a.administered_at).getTime() - new Date(b.administered_at).getTime()
    );

    // Find nearest point
    let nearestScore: OutcomeScore | null = null;
    let minDistance = Infinity;
    let nearestX = 0;
    let nearestY = 0;

    sortedScores.forEach((score, index) => {
      const x = padding.left + (chartWidth / Math.max(sortedScores.length - 1, 1)) * index;
      const y = padding.top + chartHeight * (1 - score.score / maxScore);
      const distance = Math.sqrt(Math.pow(mouseX - x, 2) + Math.pow(mouseY - y, 2));

      if (distance < minDistance && distance < 20) {
        minDistance = distance;
        nearestScore = score;
        nearestX = x;
        nearestY = y;
      }
    });

    if (nearestScore) {
      hoveredPoint = { score: nearestScore, x: nearestX, y: nearestY };
    } else {
      hoveredPoint = null;
    }
  };

  const handleMouseLeave = () => {
    hoveredPoint = null;
  };

  onMount(() => {
    const resizeCanvas = () => {
      const container = canvas.parentElement;
      if (container) {
        canvas.width = container.clientWidth;
        canvas.height = 400;
        drawChart();
      }
    };

    resizeCanvas();
    window.addEventListener('resize', resizeCanvas);

    return () => {
      window.removeEventListener('resize', resizeCanvas);
    };
  });

  $effect(() => {
    // Redraw when scores change
    scores;
    drawChart();
  });
</script>

<div class="relative w-full">
  <canvas
    bind:this={canvas}
    onmousemove={handleMouseMove}
    onmouseleave={handleMouseLeave}
    class="w-full cursor-crosshair"
  ></canvas>

  {#if hoveredPoint}
    <div
      class="absolute bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg shadow-lg p-3 pointer-events-none z-10"
      style="left: {hoveredPoint.x + 10}px; top: {hoveredPoint.y - 60}px;"
    >
      <div class="text-sm">
        <div class="font-semibold text-gray-900 dark:text-gray-100">
          Score: {hoveredPoint.score.score}
        </div>
        <div class="text-gray-600 dark:text-gray-400">
          {hoveredPoint.score.interpretation}
        </div>
        <div class="text-xs text-gray-500 dark:text-gray-500 mt-1">
          {formatDate(hoveredPoint.score.administered_at)}
        </div>
      </div>
    </div>
  {/if}
</div>
