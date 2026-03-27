## Basic Radar Diagram

```mermaid
radar-beta
  axis A, B, C, D, E
  curve c1{1, 2, 3, 4, 5}
  curve c2{5, 4, 3, 2, 1}
```

## Radar Diagram with Title

```mermaid
radar-beta
  title Skills Assessment
  axis Coding, Design, Communication, Leadership, Teamwork
  curve c1{4, 3, 5, 2, 4}
```

## Radar Diagram with Labeled Axes

```mermaid
radar-beta
  title Product Comparison
  axis perf["Performance"], rel["Reliability"], usab["Usability"], sec["Security"], scal["Scalability"]
  curve p1["Product A"]{4, 3, 5, 4, 2}
  curve p2["Product B"]{3, 5, 3, 5, 4}
```

## Radar Diagram with Multiple Curves

```mermaid
radar-beta
  title Team Comparison
  axis Technical, Creative, Analytical, Social, Management
  curve t1["Team Alpha"]{5, 3, 4, 2, 3}
  curve t2["Team Beta"]{3, 5, 2, 4, 4}
  curve t3["Team Gamma"]{4, 4, 5, 3, 2}
```

## Radar Diagram with Multiple Curves on Single Line

```mermaid
radar-beta
  axis A, B, C
  curve c1["First"]{1, 2, 3}, c2["Second"]{3, 2, 1}
```

## Radar Diagram with Key-Value Curve Definition

```mermaid
radar-beta
  axis speed, quality, cost
  curve c1{ cost: 30, speed: 20, quality: 10 }
```

## Radar Diagram with showLegend Disabled

```mermaid
radar-beta
  title No Legend
  axis A, B, C, D
  curve c1{3, 4, 2, 5}
  curve c2{5, 2, 4, 3}
  showLegend false
```

## Radar Diagram with Custom Max and Min

```mermaid
radar-beta
  title Scaled Diagram
  axis A, B, C, D, E
  curve c1{20, 40, 60, 80, 100}
  max 100
  min 0
```

## Radar Diagram with Polygon Graticule

```mermaid
radar-beta
  title Polygon Graticule
  axis A, B, C, D, E, F
  curve c1{3, 4, 2, 5, 1, 4}
  graticule polygon
```

## Radar Diagram with Circle Graticule

```mermaid
radar-beta
  title Circle Graticule
  axis A, B, C, D, E, F
  curve c1{3, 4, 2, 5, 1, 4}
  graticule circle
```

## Radar Diagram with Custom Ticks

```mermaid
radar-beta
  title Custom Ticks
  axis A, B, C, D
  curve c1{2, 4, 6, 8}
  ticks 8
  max 10
```

## Radar Diagram with All Options

```mermaid
radar-beta
  title Full Options Example
  axis Strength, Speed, Endurance, Intelligence, Agility
  curve hero["Hero"]{80, 60, 70, 90, 85}
  curve villain["Villain"]{70, 80, 60, 75, 90}
  showLegend true
  max 100
  min 0
  graticule circle
  ticks 5
```

## Radar Diagram with Theme Color Scales

```mermaid
---
config:
  themeVariables:
    cScale0: "#FF0000"
    cScale1: "#00FF00"
---
radar-beta
  title Themed Colors
  axis A, B, C, D, E
  curve c1{4, 3, 5, 2, 4}
  curve c2{2, 5, 3, 4, 3}
```

## Radar Diagram with Radar Style Options

```mermaid
---
config:
  themeVariables:
    radar:
      axisColor: "#FF0000"
      graticuleColor: "#CCCCCC"
      curveOpacity: 0.5
      curveStrokeWidth: 3
---
radar-beta
  title Custom Radar Styles
  axis A, B, C, D, E
  curve c1{5, 3, 4, 2, 5}
  curve c2{3, 5, 2, 4, 3}
  graticule polygon
  ticks 4
```
