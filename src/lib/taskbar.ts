import { useSyncExternalStore } from "react";

export interface ActiveTask {
  id: string;
  label: string;
  percent: number;
  onCancel: () => void;
}

let current: ActiveTask | null = null;
const listeners = new Set<() => void>();

function emit() {
  for (const listener of listeners) listener();
}

export function setActiveTask(task: ActiveTask | null) {
  current = task;
  emit();
}

export function updateActiveTask(percent: number) {
  if (!current) return;
  current = { ...current, percent };
  emit();
}

export function getActiveTask() {
  return current;
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useActiveTask(): ActiveTask | null {
  return useSyncExternalStore(subscribe, getActiveTask, getActiveTask);
}
