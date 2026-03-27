## Basic Mindmap

```mermaid
mindmap
  Root
    A
      B
      C
```

## Default Shape

```mermaid
mindmap
  Root
    A
    B
    C
```

## Square Shape

```mermaid
mindmap
  id[I am a square]
    id2[I am a square]
      id3[I am a square]
```

## Rounded Square Shape

```mermaid
mindmap
  id(I am a rounded square)
    id2(I am a rounded square)
      id3(I am a rounded square)
```

## Circle Shape

```mermaid
mindmap
  id((I am a circle))
    id2((I am a circle))
      id3((I am a circle))
```

## Bang Shape

```mermaid
mindmap
  id))I am a bang((
    id2))I am a bang((
      id3))I am a bang((
```

## Cloud Shape

```mermaid
mindmap
  id)I am a cloud(
    id2)I am a cloud(
      id3)I am a cloud(
```

## Hexagon Shape

```mermaid
mindmap
  id{{I am a hexagon}}
    id2{{I am a hexagon}}
      id3{{I am a hexagon}}
```

## Icons

```mermaid
mindmap
  Root
    A
      ::icon(fa fa-book)
      B(B)
        ::icon(mdi mdi-skull-outline)
    C
      ::icon(fa fa-twitter)
```

## Classes

```mermaid
mindmap
  Root
    A[A]
      B[B]
      C[C]
        ::icon(fa fa-book)
        D[Important]:::urgent
```

## Unclear Indentation

```mermaid
mindmap
  Root
    A
      B
    C
```

## Markdown Strings

```mermaid
mindmap
  id["`**Root**`"]
    id2["`**Bold item**
    *Italic item*
    Normal text`"]
      id3["`*Another item*`"]
```

## Tidy Tree Layout

```mermaid
---
config:
  layout: tidy-tree
---
mindmap
root((mindmap is a long thing))
  A
  B
  C
  D
```

## Comprehensive Mindmap Example

```mermaid
mindmap
  root((mindmap))
    Origins
      Long history
      ::icon(fa fa-book)
      Popularisation
        British popular psychology author Tony Buzan
    Research
      On effectiveness<br/>and features
      On Automatic creation
        Uses
          Creative techniques
          Strategic planning
          Argument mapping
    Tools
      Pen and paper
      Mermaid
```
