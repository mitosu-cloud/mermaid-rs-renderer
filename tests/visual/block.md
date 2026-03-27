## Introduction to Block Diagrams

```mermaid
block
columns 1
 db(("DB"))
 blockArrowId6<[" "]>(down)
 block:ID
 A
 B["A wide one in the middle"]
 C
 end
 space
 D
 ID --> D
 C --> D
 style B fill:#969,stroke:#333,stroke-width:4px
```

## Simple Block Diagram

```mermaid
block
 a b c
```

## Multi-Column Diagram

```mermaid
block
 columns 3
 a b c d
```

## Block Spanning Multiple Columns

```mermaid
block
 columns 3
 a["A label"] b:2 c:2 d
```

## Composite Blocks

```mermaid
block
 block
 D
 end
 A["A: I am a wide one"]
```

## Dynamic Column Widths

```mermaid
block
 columns 3
 a:3
 block:group1:2
 columns 2
 h i j k
 end
 g
 block:group2:3
 %% columns auto (default)
 l m n o p q r
 end
```

## Merging Blocks Horizontally

```mermaid
block
 block
 columns 1
 a["A label"] b c d
 end
```

## Round Edged Block

```mermaid
block
 id1("This is the text in the box")
```

## Stadium-Shaped Block

```mermaid
block
 id1(["This is the text in the box"])
```

## Subroutine Shape

```mermaid
block
 id1[["This is the text in the box"]]
```

## Cylindrical Shape

```mermaid
block
 id1[("Database")]
```

## Circle Shape

```mermaid
block
 id1(("This is the text in the circle"))
```

## Asymmetric Shape

```mermaid
block
 id1>"This is the text in the box"]
```

## Rhombus Shape

```mermaid
block
 id1{"This is the text in the box"}
```

## Hexagon Shape

```mermaid
block
 id1{{"This is the text in the box"}}
```

## Parallelogram and Trapezoid Shapes

```mermaid
block
 id1[/"This is the text in the box"/]
 id2[\"This is the text in the box"\]
 A[/"Christmas"\]
 B[\"Go shopping"/]
```

## Double Circle

```mermaid
block
 id1((("This is the text in the circle")))
```

## Block Arrows

```mermaid
block
 blockArrowId<["Label"]>(right)
 blockArrowId2<["Label"]>(left)
 blockArrowId3<["Label"]>(up)
 blockArrowId4<["Label"]>(down)
 blockArrowId5<["Label"]>(x)
 blockArrowId6<["Label"]>(y)
 blockArrowId7<["Label"]>(x, down)
```

## Space Blocks

```mermaid
block
 columns 3
 a space b
 c d e
```

## Space Blocks with Width

```mermaid
block
 ida space:3 idb idc
```

## Basic Links

```mermaid
block
 A space B
 A-->B
```

## Text with Links

```mermaid
block
 A space:2 B
 A-- "X" -->B
```

## Edges and Styles

```mermaid
block
columns 1
 db(("DB"))
 blockArrowId6<[" "]>(down)
 block:ID
 A
 B["A wide one in the middle"]
 C
 end
 space
 D
 ID --> D
 C --> D
 style B fill:#939,stroke:#333,stroke-width:4px
```

## Individual Block Styling

```mermaid
block
 id1 space id2
 id1("Start")-->id2("Stop")
 style id1 fill:#636,stroke:#333,stroke-width:4px
 style id2 fill:#bbf,stroke:#f66,stroke-width:2px,color:#fff,stroke-dasharray: 5 5
```

## Class Styling

```mermaid
block
 A space B
 A-->B
 classDef blue fill:#6e6ce6,stroke:#333,stroke-width:4px;
 class A blue
 style B fill:#bbf,stroke:#f66,stroke-width:2px,color:#fff,stroke-dasharray: 5 5
```

## System Architecture

```mermaid
block
 columns 3
 Frontend blockArrowId6<[" "]>(right) Backend
 space:2 down<[" "]>(down)
 Disk left<[" "]>(left) Database[("Database")]

 classDef front fill:#696,stroke:#333;
 classDef back fill:#969,stroke:#333;
 class Frontend front
 class Backend,Database back
```
