// ECharts por partes (ADR-0005 §2): solo barras, con el renderizador SVG (texto nítido e impresión vectorial).
import { BarChart } from "echarts/charts";
import { AriaComponent, GridComponent, LegendComponent, TooltipComponent } from "echarts/components";
import * as echarts from "echarts/core";
import { SVGRenderer } from "echarts/renderers";

echarts.use([BarChart, GridComponent, TooltipComponent, LegendComponent, AriaComponent, SVGRenderer]);

export { echarts };
