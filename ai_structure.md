# AI Agents Strategy

## **AI Agent Architecture Overview**

The AI system will be divided into several specialized agents, each with a distinct role. They will work together asynchronously to manage the game state, maintain narrative consistency, and respond to player actions.

---

## 1. **The Visionary Archivist**

- **Role:** Passive background agent.
- **Function:** Continuously listens to new messages and updates the long-term memory and narrative context.
- **Responsibilities:**

  - Decides what is worth remembering.
  - Tracks character motivations, hidden intentions, potential betrayals, and story arcs.
  - Predicts narrative developments to maintain dramatic tension and coherence.
  - Outputs summaries and narrative forecasts to help the next agent make informed decisions.

---

## 2. **The Strategist**

- **Role:** Central decision-maker.
- **Function:** Receives the player’s input, game rules, and all contextual data to decide the next step.
- **Input:**

  - Player’s request.
  - Summaries from the Visionary Archivist.
  - Current game state: character stats, inventories, rule status.
  - Last five messages in the conversation (for immediate context).

- **Responsibilities:**

  - Determines what should happen next in the game world.
  - Outputs:

    1. **Game logic operations** (e.g. stat updates, dice rolls).
    2. **Story content** (description of the event that will unfold).

---

## 3. **The Tool Handler**

- **Role:** Executes backend operations.
- **Function:** Receives instructions from the Strategist about mechanical actions.
- **Responsibilities:**

  - Updates character sheets.
  - Modifies inventories.
  - Initiates dice rolls or other rule-based mechanics.
  - Waits for dice results if needed before continuing.
  - Works in parallel with the narrator if no blocking operations are required.

---

## 4. **The Narrator**

- **Role:** Story generator.
- **Function:** Produces the player-facing narrative output based on instructions and context.
- **Input:**

  - Immediate scene instructions from the Strategist.
  - Relevant historical context from the Archivist.

- **Responsibilities:**

  - Writes immersive narration consistent with the game's tone and events.
  - Optionally outputs audio or streaming content.

---

This separation should allow each agent to focus on its domain (memory, logic, mechanics, storytelling), resulting in a responsive, coherent, and immersive gameplay experience.
