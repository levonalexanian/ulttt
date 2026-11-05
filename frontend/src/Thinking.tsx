import { useEffect, useState } from "react";

/** What the engine says it is doing while it thinks, in the style of LLM terminals. */
const WORDS = [
  "Thinking",
  "Baking",
  "Brewing",
  "Calculating",
  "Cerebrating",
  "Churning",
  "Coalescing",
  "Cogitating",
  "Computing",
  "Concocting",
  "Conjuring",
  "Considering",
  "Contemplating",
  "Cooking",
  "Crafting",
  "Crunching",
  "Deliberating",
  "Divining",
  "Fermenting",
  "Finagling",
  "Forging",
  "Hatching",
  "Ideating",
  "Incubating",
  "Inferring",
  "Manifesting",
  "Marinating",
  "Moseying",
  "Mulling",
  "Musing",
  "Noodling",
  "Percolating",
  "Pondering",
  "Processing",
  "Puttering",
  "Reticulating",
  "Ruminating",
  "Scheming",
  "Simmering",
  "Spinning",
  "Stewing",
  "Synthesizing",
  "Tinkering",
  "Transmuting",
  "Vibing",
  "Whirring",
];


export const randomWord = () => WORDS[Math.floor(Math.random() * WORDS.length)];

/** A spinner, a word and dots that count up, e.g. "✎ Pondering..". */
export function Thinking({ word }: { word: string }) {
  const [tick, setTick] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setTick((t) => t + 1), 400);
    return () => clearInterval(id);
  }, []);
  // The dots count up: "", ".", "..", "...".
  const dots = ".".repeat(tick % 4);
  return (
    <span className="thinking">
      <span className="spinner" aria-hidden="true">
        {"✎︎"}
      </span>
      {word}
      <span className="dots">{dots}</span>
    </span>
  );
}
