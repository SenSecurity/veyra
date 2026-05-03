import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { BrandMark } from "./brand-mark";

describe("BrandMark", () => {
  it("renders an inline SVG with role=presentation", () => {
    render(<BrandMark />);
    const svg = screen.getByRole("presentation", { hidden: true });
    expect(svg.tagName.toLowerCase()).toBe("svg");
  });

  it("merges custom className with default classes", () => {
    const { container } = render(<BrandMark className="h-8 w-8 rounded-md" />);
    const wrapper = container.querySelector("span.veyra-brand-mark");
    expect(wrapper).not.toBeNull();
    expect(wrapper?.className).toContain("h-8");
    expect(wrapper?.className).toContain("w-8");
    expect(wrapper?.className).toContain("rounded-md");
    expect(wrapper?.className).toContain("inline-flex");
  });

  it("does not import the legacy PNG asset", () => {
    const { container } = render(<BrandMark />);
    expect(container.querySelector("img")).toBeNull();
  });
});
