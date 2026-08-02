import type { PreviewChart, PreviewMetrics as PreviewMetricsData } from "../types";

/**
 * The quality summary above a FASTQ or VCF preview.
 *
 * Charts are inline SVG rather than a charting library: these are two shapes,
 * a line and a bar series, and the workbench already renders analysis plots as
 * SVG. Pulling in a chart dependency for this would cost more than it saves.
 */

const CHART_WIDTH = 520;
const CHART_HEIGHT = 132;
const PADDING = { top: 10, right: 8, bottom: 22, left: 34 };

function niceMax(value: number): number {
  if (value <= 0) return 1;
  const magnitude = 10 ** Math.floor(Math.log10(value));
  return Math.ceil(value / magnitude) * magnitude;
}

/** Enough tick labels to read the axis, never so many that they collide. */
function tickIndexes(count: number, maximum = 12): number[] {
  if (count <= maximum) return Array.from({ length: count }, (_, index) => index);
  const step = Math.ceil(count / maximum);
  const ticks: number[] = [];
  for (let index = 0; index < count; index += step) ticks.push(index);
  if (ticks.at(-1) !== count - 1) ticks.push(count - 1);
  return ticks;
}

function Chart({ chart }: { chart: PreviewChart }) {
  const values = chart.series[0]?.values ?? [];
  if (!values.length) return null;
  const plotWidth = CHART_WIDTH - PADDING.left - PADDING.right;
  const plotHeight = CHART_HEIGHT - PADDING.top - PADDING.bottom;
  const maximum = niceMax(Math.max(...values));
  const x = (index: number) => values.length === 1
    ? PADDING.left + plotWidth / 2
    : PADDING.left + (index / (values.length - 1)) * plotWidth;
  const y = (value: number) => PADDING.top + plotHeight - (value / maximum) * plotHeight;
  const ticks = tickIndexes(values.length);

  return (
    <figure className="preview-chart">
      <figcaption>{chart.title}</figcaption>
      <svg
        viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
        role="img"
        aria-label={`${chart.title}: ${chart.yLabel} against ${chart.xLabel}`}
        preserveAspectRatio="none"
      >
        <line
          className="preview-chart-axis"
          x1={PADDING.left}
          y1={PADDING.top}
          x2={PADDING.left}
          y2={PADDING.top + plotHeight}
        />
        <line
          className="preview-chart-axis"
          x1={PADDING.left}
          y1={PADDING.top + plotHeight}
          x2={PADDING.left + plotWidth}
          y2={PADDING.top + plotHeight}
        />
        <text className="preview-chart-tick" x={PADDING.left - 4} y={PADDING.top + 4} textAnchor="end">
          {maximum.toLocaleString()}
        </text>
        <text
          className="preview-chart-tick"
          x={PADDING.left - 4}
          y={PADDING.top + plotHeight}
          textAnchor="end"
        >0</text>

        {chart.kind === "line" ? (
          <polyline
            className="preview-chart-line"
            points={values.map((value, index) => `${x(index)},${y(value)}`).join(" ")}
          />
        ) : (
          values.map((value, index) => {
            const barWidth = Math.max(1, plotWidth / values.length - 1);
            const left = PADDING.left + (index / values.length) * plotWidth;
            return (
              <rect
                className="preview-chart-bar"
                key={chart.categories[index] ?? index}
                x={left}
                y={y(value)}
                width={barWidth}
                height={Math.max(0, PADDING.top + plotHeight - y(value))}
              >
                <title>{`${chart.categories[index] ?? index}: ${value.toLocaleString()}`}</title>
              </rect>
            );
          })
        )}

        {ticks.map((index) => (
          <text
            className="preview-chart-tick"
            key={index}
            x={chart.kind === "line"
              ? x(index)
              : PADDING.left + ((index + 0.5) / values.length) * plotWidth}
            y={CHART_HEIGHT - 8}
            textAnchor="middle"
          >{chart.categories[index] ?? index + 1}</text>
        ))}
      </svg>
      <small>{chart.xLabel}</small>
    </figure>
  );
}

export function PreviewMetricsPanel({
  metrics,
  sampled,
}: {
  metrics: PreviewMetricsData;
  sampled: boolean;
}) {
  return (
    <section className="preview-metrics" aria-label="Quality metrics">
      <div className="preview-facts">
        {metrics.facts.map((fact) => (
          <div key={fact.label}>
            <span>{fact.label}</span>
            <strong>{fact.value}</strong>
          </div>
        ))}
      </div>
      {sampled && (
        // Saying so matters: these are computed from the preview prefix, and a
        // reader who assumes they cover the file would draw wrong conclusions.
        <p className="preview-metrics-note">
          Computed from the sampled portion of the file, not the whole file.
        </p>
      )}
      <div className="preview-charts">
        {metrics.charts.map((chart) => <Chart chart={chart} key={chart.title} />)}
      </div>
    </section>
  );
}
