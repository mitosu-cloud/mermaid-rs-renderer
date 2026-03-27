## Sales Revenue Bar and Line Chart

```mermaid
xychart
    title "Sales Revenue"
    x-axis [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
    y-axis "Revenue (in $)" 4000 --> 11000
    bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]
    line [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]
```

## Simplest Line Chart

```mermaid
xychart
    line [+1.3, .6, 2.4, -.34]
```

## Different Colors with Multiple Series

```mermaid
---
config:
  themeVariables:
    xyChart:
      plotColorPalette: '#000000, #0000FF, #00FF00, #FF0000'
---
xychart
title "Different Colors in xyChart"
x-axis "categoriesX" ["Category 1", "Category 2", "Category 3", "Category 4"]
y-axis "valuesY" 0 --> 50
%% Black line
line [10,20,30,40]
%% Blue bar
bar [20,30,25,35]
%% Green bar
bar [15,25,20,30]
%% Red line
line [5,15,25,35]
```

## Data Labels Inside Bars

```mermaid
---
config:
  xyChart:
    showDataLabel: true
---
xychart
 title "Genres in top 100 book survey of 2025"
 x-axis [comedy, romance, mystery, crime, "non fiction", other]
 y-axis "Number of Books" 0 --> 30
 bar [12,2,20,25,17,24]
```

## Data Labels Outside Bars

```mermaid
---
config:
  xyChart:
    showDataLabel: true
    showDataLabelOutsideBar: true
---
xychart
 title "Genres in top 100 book survey of 2025"
 x-axis [comedy, romance, mystery, crime, "non fiction", other]
 y-axis "Number of Books" 0 --> 30
 bar [12,2,20,25,17,24]
```

## Full Configuration with Theme

```mermaid
---
config:
  xyChart:
    width: 900
    height: 600
    showDataLabel: true
  themeVariables:
    xyChart:
      titleColor: "#ff0000"
---
xychart
 title "Sales Revenue"
 x-axis [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
 y-axis "Revenue (in $)" 4000 --> 11000
 bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]
 line [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]
```
