import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Sidebar } from "./sidebar";

vi.mock("@tauri-apps/api/app", () => ({
  getVersion: () => Promise.resolve("0.9.4"),
}));

vi.mock("@tanstack/react-router", () => ({
  Link: ({
    children,
    title,
    className,
  }: {
    children?: React.ReactNode;
    title?: string;
    className?: string;
    to?: string;
    activeProps?: unknown;
    activeOptions?: unknown;
  }) => (
    <a className={className} title={title}>
      {children}
    </a>
  ),
}));

vi.mock("@/hooks/use-settings", () => ({
  useSettings: () => ({
    settings: {
      whisperModel: "large-v3-turbo",
      emailDraftEngine: "ollama",
      emailDraftModel: "llama3.2:1b",
    },
  }),
}));

describe("Sidebar", () => {
  it("renders the five primary nav items", () => {
    render(<Sidebar />);
    expect(screen.getByText("Home")).toBeInTheDocument();
    expect(screen.getByText("History")).toBeInTheDocument();
    expect(screen.getByText("Email Drafter")).toBeInTheDocument();
    expect(screen.getByText("Dictionary")).toBeInTheDocument();
    expect(screen.getByText("Settings")).toBeInTheDocument();
  });

  it("renders both engine cards with role captions", () => {
    const { container } = render(<Sidebar />);
    const text = container.textContent ?? "";
    expect(text).toMatch(/STT\s*·\s*01/);
    expect(text).toMatch(/Drafter\s*·\s*02/);
  });

  it("renders the status footer with All systems nominal", () => {
    const { container } = render(<Sidebar />);
    const text = container.textContent ?? "";
    expect(text).toMatch(/Storage/);
    expect(text).toMatch(/Models/);
    expect(text).toMatch(/All systems nominal/);
  });

  it("does not surface Cmd-symbol nav badges or %appdata% copy", () => {
    const { container } = render(<Sidebar />);
    const text = container.textContent ?? "";
    expect(text).not.toMatch(/⌘/);
    expect(text).not.toMatch(/%appdata%/i);
    expect(text).not.toMatch(/Local services running/);
  });
});
