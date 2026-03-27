## Basic User Journey

```mermaid
journey
    title My working day
    section Go to work
      Make tea: 5: Me
      Go upstairs: 3: Me
      Do work: 1: Me, Cat
    section Go home
      Go downstairs: 5: Me
      Sit down: 5: Me
```

## Multiple Sections With Multiple Actors

```mermaid
journey
    title Online Shopping Experience
    section Browse
      Visit homepage: 5: Customer
      Search for product: 4: Customer
      View product details: 3: Customer, Support
    section Purchase
      Add to cart: 5: Customer
      Enter shipping info: 2: Customer
      Enter payment info: 1: Customer
    section Delivery
      Receive confirmation: 5: Customer, Support
      Track package: 4: Customer
      Receive delivery: 5: Customer, Delivery
```

## Single Section

```mermaid
journey
    title Morning Routine
    section Getting Ready
      Wake up: 1: Me
      Brush teeth: 3: Me
      Shower: 4: Me
      Get dressed: 3: Me
      Eat breakfast: 5: Me
```

## Tasks With Varying Scores

```mermaid
journey
    title Score Range Demo
    section Tasks
      Terrible task: 1: User
      Bad task: 2: User
      Okay task: 3: User
      Good task: 4: User
      Great task: 5: User
```

## Multiple Actors Per Task

```mermaid
journey
    title Team Collaboration
    section Planning
      Define requirements: 4: Dev, PM, Designer
      Create mockups: 3: Designer
      Review mockups: 4: Dev, PM, Designer
    section Development
      Write code: 3: Dev
      Code review: 4: Dev, PM
      Deploy: 2: Dev, Ops
```

## Accessible Title and Description

```mermaid
journey
    accTitle: My daily workflow diagram
    accDescr: A user journey showing the steps in a typical work day
    title My Daily Workflow
    section Morning
      Check email: 3: Me
      Stand-up meeting: 4: Me, Team
    section Afternoon
      Deep work: 5: Me
      Review PRs: 4: Me, Team
```

## Multiline Accessible Description

```mermaid
journey
    accTitle: Customer onboarding journey
    accDescr {
        This diagram shows the complete
        customer onboarding process from
        signup through first use.
    }
    title Customer Onboarding
    section Signup
      Create account: 4: Customer
      Verify email: 3: Customer
    section Setup
      Complete profile: 2: Customer
      Connect integrations: 1: Customer, Support
    section First Use
      View tutorial: 4: Customer
      Complete first task: 5: Customer
```

## Title Only With Tasks

```mermaid
journey
    title Simple Journey
    section Steps
      First step: 5: Alice
      Second step: 3: Alice, Bob
      Third step: 4: Bob
```
