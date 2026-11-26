import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

// Canvas rendering is verified in the real-browser visual suite, not jsdom.
HTMLCanvasElement.prototype.getContext = vi.fn().mockReturnValue(null);

afterEach(() => cleanup());
