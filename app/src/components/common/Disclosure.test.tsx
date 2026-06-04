import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, test } from "vitest";

import { Disclosure } from "./Disclosure";

describe("Disclosure", () => {
  test("is collapsed by default and toggles open/closed accessibly", () => {
    render(
      <Disclosure id="x" title="Section X" count={3}>
        <p>body content</p>
      </Disclosure>,
    );
    const toggle = screen.getByTestId("disclosure-x-toggle");
    // Collapsed: aria-expanded=false, panel hidden, count rendered in the header.
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByText("3")).toBeInTheDocument();
    // The panel exists but is hidden (children always mounted in non-lazy mode).
    expect(screen.getByRole("region", { hidden: true })).toHaveAttribute("hidden");

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("region")).not.toHaveAttribute("hidden");

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
  });

  test("respects defaultOpen", () => {
    render(
      <Disclosure id="y" title="Section Y" defaultOpen>
        <p>open body</p>
      </Disclosure>,
    );
    expect(screen.getByTestId("disclosure-y-toggle")).toHaveAttribute("aria-expanded", "true");
  });

  test("lazy children do not mount until first opened, then stay mounted", () => {
    render(
      <Disclosure id="z" title="Heavy" lazy>
        <p data-testid="heavy-body">heavy body</p>
      </Disclosure>,
    );
    // Collapsed + lazy => the heavy child is NOT in the DOM (no idle cost).
    expect(screen.queryByTestId("heavy-body")).toBeNull();

    const toggle = screen.getByTestId("disclosure-z-toggle");
    fireEvent.click(toggle); // open -> mounts
    expect(screen.getByTestId("heavy-body")).toBeInTheDocument();

    fireEvent.click(toggle); // collapse again -> stays mounted (just hidden)
    expect(screen.getByTestId("heavy-body")).toBeInTheDocument();
  });
});
