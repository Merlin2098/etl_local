# Diseño de interfaz

> Estado: **aprobado** (mocks v2) · Fecha: 2026-11-16
> Mocks: lienzo de diseño privado del autor ([enlace](https://claude.ai/artifact/3LBpfLJmWw32hst8HVY9Nf)); hay que compartirlo para que otros lo vean.

## Para quién diseñamos

**Usuario principal:** personal administrativo de RR. HH. que solo maneja Excel. No conoce términos
como "ETL", "Parquet" o "pipeline", y lo que le importa es que el reporte quede bien.

**Usuarios secundarios:**

- **Dirección (CEO/CHRO):** mira el tablero en 30 segundos y necesita tasas, tendencias y riesgos, no listas.
- **Analista:** opera la herramienta y tiene que **explicar** cada número (fuente, fecha de corte, regla).

## Principios

1. **Hablar el idioma de Excel.** Los errores se ubican en hoja, fila y celda (`Hoja «Planilla» · celda A6`),
   con un recorte visual de la hoja y pasos para arreglarlo.
2. **Un camino y un botón principal por pantalla.** Pasos numerados visibles (Elegir → Revisar → Procesar → Listo).
   Un botón desactivado siempre dice por qué lo está.
3. **Cero jerga.** La UI no muestra Bronze/Silver/Gold, ETL, Parquet ni preflight. Esos términos quedan para
   "Ver detalles para soporte".
4. **Cada número es explicable.** Botón "¿De dónde sale?" y página "Cómo leer este tablero" (TR-D17).
5. **Tasas antes que cantidades.** Cada indicador se compara con el mes anterior y con una meta, si existe
   ([indicadores-tablero.md](../logica-negocio/indicadores-tablero.md)).
6. **Mostrar la frescura de los datos.** La barra "Datos usados" avisa cuando hay periodos mezclados (TR-D16).
7. **Privacidad por nivel.** Los nombres de personas solo aparecen en vistas para RR. HH. (TR-D18).

## Pantallas (mocks v2)

| # | Pantalla | Propósito |
|---|---|---|
| 1 | Inicio | Elegir qué reporte actualizar y ver su estado; acceso al tablero |
| 2 | Elegir archivos | Arrastrar Excel; se reconoce el mes por el nombre; opción de sumar licencias |
| 3 | Revisión | Resultado por archivo; error explicado con celdas y un recorte de la hoja (TR-D13) |
| 4 | Procesando | Tareas en lenguaje de oficina, con avance y tiempo estimado |
| 5 | Listo | Cifras clave, avisos "Para revisar" (TR-D14) y acciones: tablero, Excel o carpeta |
| 6 | Tablero | Datos usados, filtros, 4 indicadores en tasa, riesgos agregados y gráficos con alcance de filtros |
| 7 | Cómo leer este tablero | Definiciones, por qué hay dos cifras y qué filtros afectan a cada gráfico |

**Implementado (Plan 6, 2026-11-16):** pantallas 1–5 en `app/ui/` (Svelte 5), con los textos de los mocks.

- **Textos ajustados:** saludo según la hora; textos de la base de personal (fecha de corte en lugar de mes); «Mes no reconocido»; «Todo listo para procesar».
- **Plan 7:** «Ver el tablero» (Inicio) y «Ver en el tablero» (Listo, acción principal) abren el tablero en su propia ventana, servida por la API local como en el navegador. «Ver quiénes son» no se agregó: ningún aviso de Listo tiene hoy un detalle nominal asociado.
- **Fuera de alcance:** «¿Necesitas ayuda?», que no tiene contenido todavía.
- **Tarjetas de inicio:** solo aparecen los reportes disponibles, definidos en `config/reportes.yaml`.

**Implementado (Plan 7, 2026-11-17):** pantallas 6 y 7 en `app/ui/tablero.html` (Resumen, Alertas y «Cómo leer este tablero»).

- **Gráficos:** «Entradas y salidas por mes» usa ECharts (SVG). «Personas en planilla por gerencia» y «Días de ausencia» son listas de barras en HTML, como en el mock: cada barra de gerencia es un botón que filtra todo el tablero.

## Lenguaje visual

| Elemento | Valor |
|---|---|
| Tipografía de texto | Atkinson Hyperlegible, 16–17 px base (diseñada para máxima legibilidad) |
| Tipografía de títulos | Bricolage Grotesque, 700 |
| Referencias de celda y regla | JetBrains Mono |
| Fondo / superficie | `#F5F6F2` / `#FFFFFF` |
| Texto / texto secundario | `#1B2420` / `#55605A` |
| Acción principal | `#1E6B47` (verde), texto blanco |
| Aviso | fondo `#FDF1DC`, texto `#6B3A00`, siempre con ícono y palabra |
| Error | borde y texto `#A3261B` sobre `#FBE7E4` |
| Serie "Ingresaron" / "Salieron" | `#2B6CB0` / `#C2620F` (validadas para daltonismo) |
| Series únicas (barras) | `#1E6B47` |
| Controles | alto mínimo de 44 px; botón principal de 52 px |
| Elemento distintivo | Chip de celda y recorte de hoja de Excel en los errores |

## Pendientes

- Modo oscuro: definir y validar la paleta contra la superficie oscura.
- Pantallas no dibujadas: pestañas Ausentismo, Horas extra, Practicantes, Ceses y Alertas; ajustes (carpeta de salida, metas).
- Prototipo interactivo y prueba con 3–5 usuarios reales de RR. HH.
- Nombre definitivo de la aplicación ("Integrador RR. HH." es provisional).
