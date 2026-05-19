import { useContext } from "react";
import { ExperienceContext } from "./ExperienceProvider";

export function useExperience() {
  const ctx = useContext(ExperienceContext);
  if (!ctx) throw new Error("useExperience must be used within <ExperienceProvider>");
  return ctx;
}

