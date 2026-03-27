## Animal Hierarchy with Inheritance

```mermaid
---
title: Animal example
---
classDiagram
    note "From Duck till Zebra"
    Animal <|-- Duck
    note for Duck "can fly<br>can swim<br>can dive<br>can help in debugging"
    Animal <|-- Fish
    Animal <|-- Zebra
    Animal : +int age
    Animal : +String gender
    Animal: +isMammal()
    Animal: +mate()
    class Duck{
        +String beakColor
        +swim()
        +quack()
    }
    class Fish{
        -int sizeInFeet
        -canEat()
    }
    class Zebra{
        +bool is_wild
        +run()
    }
```

## Bank Account Class with Members

```mermaid
---
title: Bank example
---
classDiagram
    class BankAccount
    BankAccount : +String owner
    BankAccount : +Bigdecimal balance
    BankAccount : +deposit(amount)
    BankAccount : +withdrawal(amount)
```

## Basic Class Definition

```mermaid
classDiagram
    class Animal
    Vehicle <|-- Car
```

## Class with Labels

```mermaid
classDiagram
    class Animal["Animal with a label"]
    class Car["Car with *! symbols"]
    Animal --> Car
```

## Class Labels with Backticks

```mermaid
classDiagram
    class `Animal Class!`
    class `Car Class`
    `Animal Class!` --> `Car Class`
```

## Members Defined with Colon Syntax

```mermaid
classDiagram
    class BankAccount
    BankAccount : +String owner
    BankAccount : +BigDecimal balance
    BankAccount : +deposit(amount)
    BankAccount : +withdrawal(amount)
```

## Members Defined with Curly Braces

```mermaid
classDiagram
    class BankAccount{
        +String owner
        +BigDecimal balance
        +deposit(amount)
        +withdrawal(amount)
    }
```

## Methods with Return Types

```mermaid
classDiagram
    class BankAccount{
        +String owner
        +BigDecimal balance
        +deposit(amount) bool
        +withdrawal(amount) int
    }
```

## Generic Types in Class Definition

```mermaid
classDiagram
    class Square~Shape~{
        int id
        List~int~ position
        setPoints(List~int~ points)
        getPoints() List~int~
    }

    Square : -List~string~ messages
    Square : +setMessages(List~string~ messages)
    Square : +getMessages() List~string~
    Square : +getDistanceMatrix() List~List~int~~
```

## Relationship Types Demonstration

```mermaid
classDiagram
    classA <|-- classB
    classC *-- classD
    classE o-- classF
    classG <-- classH
    classI -- classJ
    classK <.. classL
    classM <|.. classN
    classO .. classP
```

## Relationships with Labels

```mermaid
classDiagram
    classA --|> classB : Inheritance
    classC --* classD : Composition
    classE --o classF : Aggregation
    classG --> classH : Association
    classI -- classJ : Link(Solid)
    classK ..> classL : Dependency
    classM ..|> classN : Realization
    classO .. classP : Link(Dashed)
```

## Labels on Relations

```mermaid
classDiagram
    classA <|-- classB : implements
    classC *-- classD : composition
    classE o-- classF : aggregation
```

## Two-Way Relations

```mermaid
classDiagram
    Animal <|--|> Zebra
```

## Lollipop Interface Simple

```mermaid
classDiagram
    bar ()-- foo
```

## Lollipop Interface Complex

```mermaid
classDiagram
    class Class01 {
        int amount
        draw()
    }
    Class01 --() bar
    Class02 --() bar

    foo ()-- Class01
```

## Namespace Grouping

```mermaid
classDiagram
    namespace BaseShapes {
        class Triangle
        class Rectangle {
            double width
            double height
        }
    }
```

## Cardinality on Relations

```mermaid
classDiagram
    Customer "1" --> "*" Ticket
    Student "1" --> "1..*" Course
    Galaxy --> "many" Star : Contains
```

## Annotations - Interface

```mermaid
classDiagram
    class Shape <<interface>>
```

## Annotations - Separate Line

```mermaid
classDiagram
    class Shape
    <<interface>> Shape
    Shape : noOfVertices
    Shape : draw()
```

## Annotations - Nested Structure with Enumeration

```mermaid
classDiagram
    class Shape{
        <<interface>>
        noOfVertices
        draw()
    }
    class Color{
        <<enumeration>>
        RED
        BLUE
        GREEN
        WHITE
        BLACK
    }
```

## Diagram Direction Right-to-Left

```mermaid
classDiagram
    direction RL
    class Student {
        -idCard : IdCard
    }
    class IdCard{
        -id : int
        -name : string
    }
    class Bike{
        -id : int
        -name : string
    }
    Student "1" --o "1" IdCard : carries
    Student "1" --o "1" Bike : rides
```

## Notes on Diagram

```mermaid
classDiagram
    note "This is a general note"
    note for MyClass "This is a note for a class"
    class MyClass{
    }
```

## Interactive Links

```mermaid
classDiagram
    class Shape
    link Shape "https://www.github.com" "This is a tooltip for a link"
    class Shape2
    click Shape2 href "https://www.github.com" "This is a tooltip for a link"
```

## Interactive Callbacks

```mermaid
classDiagram
    class Shape
    callback Shape "callbackFunction" "This is a tooltip for a callback"
    class Shape2
    click Shape2 call callbackFunction() "This is a tooltip for a callback"
```

## Combined Interactions

```mermaid
classDiagram
    class Class01
    class Class02
    callback Class01 "callbackFunction" "Callback tooltip"
    link Class02 "https://www.github.com" "This is a link"
    class Class03
    class Class04
    click Class03 call callbackFunction() "Callback tooltip"
    click Class04 href "https://www.github.com" "This is a link"
```

## Styling Individual Nodes

```mermaid
classDiagram
    class Animal
    class Mineral
    style Animal fill:#f9f,stroke:#333,stroke-width:4px
    style Mineral fill:#bbf,stroke:#f66,stroke-width:2px,color:#fff,stroke-dasharray: 5 5
```

## CSS Classes Styling

```mermaid
classDiagram
    class Animal:::styleClass
    class Mineral:::styleClass2

    classDef styleClass fill:#f9f,stroke:#333,stroke-width:4px
    classDef styleClass2 fill:#bbf,stroke:#f66,stroke-width:2px
```

## Default Class Styling

```mermaid
classDiagram
    class Animal
    class Mineral

    classDef default fill:#f9f,stroke:#333,stroke-width:4px
```

## Static and Abstract Members

```mermaid
classDiagram
    class Shape {
        someAbstractMethod()*
        someStaticMethod()$
        String someField$
    }
```

## Protected and Package Visibility

```mermaid
classDiagram
    class BankAccount {
        #String accountNumber
        ~int internalId
    }
```
