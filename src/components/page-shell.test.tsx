import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { PageShell, Panel } from "./page-shell";

afterEach(() => cleanup());

describe("PageShell", () => {
  it("renders title and description (regression with prior signature)", () => {
    render(
      <PageShell title="Home" description="Quick controls.">
        <div>body</div>
      </PageShell>,
    );
    expect(screen.getByRole("heading", { name: "Home", level: 1 })).toBeInTheDocument();
    expect(screen.getByText("Quick controls.")).toBeInTheDocument();
  });

  it("renders an eyebrow caption above the title when provided", () => {
    render(
      <PageShell eyebrow="Workspace · Home" title="Home">
        <div>body</div>
      </PageShell>,
    );
    expect(screen.getByText("Workspace · Home")).toBeInTheDocument();
  });

  it("accepts a ReactNode title with an italic accent child", () => {
    render(
      <PageShell
        title={
          <>
            Boa tarde. <em data-testid="accent">quiet desk.</em>
          </>
        }
      >
        <div>body</div>
      </PageShell>,
    );
    const heading = screen.getByRole("heading", { level: 1 });
    expect(heading.textContent).toContain("Boa tarde.");
    expect(screen.getByTestId("accent").tagName.toLowerCase()).toBe("em");
  });
});

describe("Panel", () => {
  it("renders eyebrow + title together", () => {
    render(
      <Panel eyebrow="01 · Capture" title="Speech to Text">
        <div>body</div>
      </Panel>,
    );
    expect(screen.getByText("01 · Capture")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Speech to Text", level: 2 })).toBeInTheDocument();
  });

  it("renders body without a header when no metadata is passed", () => {
    render(
      <Panel>
        <p>just body</p>
      </Panel>,
    );
    expect(screen.queryByRole("heading")).toBeNull();
    expect(screen.getByText("just body")).toBeInTheDocument();
  });
});
