---
name: accessibility
description: Web accessibility patterns and WCAG compliance guidelines. Use when implementing accessible UI components, keyboard navigation, screen reader support, or auditing for a11y compliance. Triggers: accessibility, a11y, WCAG, ARIA, screen reader, keyboard navigation, color contrast, semantic HTML, assistive technology.
---

# Accessibility

## Overview

Web accessibility ensures that websites and applications are usable by everyone, including people with disabilities. This skill covers WCAG 2.1 guidelines, semantic HTML, ARIA attributes, keyboard navigation, screen reader testing, visual accessibility, and automated testing tools.

## Instructions

### 1. WCAG 2.1 Guidelines

WCAG is organized around four principles (POUR):

| Principle          | Description                                       | Key Guidelines                                 |
| ------------------ | ------------------------------------------------- | ---------------------------------------------- |
| **Perceivable**    | Information must be presentable to users          | Text alternatives, captions, adaptable content |
| **Operable**       | Interface must be operable                        | Keyboard accessible, enough time, no seizures  |
| **Understandable** | Information and operation must be understandable  | Readable, predictable, input assistance        |
| **Robust**         | Content must be robust for assistive technologies | Compatible with current and future tools       |

#### Conformance Levels

- **Level A**: Minimum accessibility (must have)
- **Level AA**: Addresses major barriers (legal requirement in many jurisdictions)
- **Level AAA**: Highest level (nice to have for specific audiences)

```typescript
// WCAG 2.1 checklist implementation
interface WCAGChecklistItem {
  id: string;
  level: "A" | "AA" | "AAA";
  principle: "perceivable" | "operable" | "understandable" | "robust";
  description: string;
  howToTest: string;
}

const wcagChecklist: WCAGChecklistItem[] = [
  {
    id: "1.1.1",
    level: "A",
    principle: "perceivable",
    description:
      "Non-text Content: All non-text content has a text alternative",
    howToTest: "Check all images have meaningful alt text",
  },
  {
    id: "1.4.3",
    level: "AA",
    principle: "perceivable",
    description:
      "Contrast (Minimum): Text has contrast ratio of at least 4.5:1",
    howToTest: "Use contrast checker tool on all text elements",
  },
  {
    id: "2.1.1",
    level: "A",
    principle: "operable",
    description: "Keyboard: All functionality is operable via keyboard",
    howToTest: "Tab through entire page without using mouse",
  },
  // ... additional items
];
```

### 2. Semantic HTML

```html
<!-- Bad: Non-semantic structure -->
<div class="header">
  <div class="nav">
    <div class="nav-item">Home</div>
    <div class="nav-item">About</div>
  </div>
</div>
<div class="main-content">
  <div class="article">
    <div class="title">Article Title</div>
    <div class="content">Content here...</div>
  </div>
</div>
<div class="footer">Footer content</div>

<!-- Good: Semantic structure -->
<header>
  <nav aria-label="Main navigation">
    <ul>
      <li><a href="/">Home</a></li>
      <li><a href="/about">About</a></li>
    </ul>
  </nav>
</header>
<main>
  <article>
    <h1>Article Title</h1>
    <p>Content here...</p>
  </article>
</main>
<footer>Footer content</footer>
```

#### Semantic Element Reference

```typescript
// Semantic HTML element mapping
const semanticElements = {
  // Sectioning
  header: "Introductory content, typically contains navigation",
  nav: "Navigation links",
  main: "Main content of the document (only one per page)",
  article: "Self-contained content that could be distributed independently",
  section: "Thematic grouping of content with a heading",
  aside: "Content tangentially related to the main content",
  footer: "Footer for its nearest sectioning content",

  // Text content
  h1_h6: "Heading levels (maintain hierarchy, one h1 per page)",
  p: "Paragraph",
  ul_ol: "Unordered/ordered lists",
  blockquote: "Extended quotation",
  figure: "Self-contained content with optional caption",
  figcaption: "Caption for a figure",

  // Interactive
  button: "Clickable button (not for links)",
  a: "Hyperlink to another page or resource",
  details: "Disclosure widget with summary",
  dialog: "Modal or non-modal dialog box",

  // Form elements
  form: "Interactive form",
  label: "Caption for form element (always use with inputs)",
  fieldset: "Group of related form elements",
  legend: "Caption for fieldset",
};
```

### 3. ARIA Attributes

```typescript
// ARIA roles, states, and properties

// Landmark roles
const landmarkRoles = [
  "banner", // Page header (use with <header>)
  "navigation", // Navigation (use with <nav>)
  "main", // Main content (use with <main>)
  "complementary", // Supporting content (use with <aside>)
  "contentinfo", // Footer (use with <footer>)
  "search", // Search functionality
  "form", // Form (use with <form>)
  "region", // Generic landmark (requires aria-label)
];

// Widget roles
const widgetRoles = [
  "button",
  "checkbox",
  "dialog",
  "menu",
  "menuitem",
  "progressbar",
  "slider",
  "tab",
  "tablist",
  "tabpanel",
  "tooltip",
  "tree",
  "treeitem",
];

// Common ARIA attributes
interface AriaAttributes {
  // Labels and descriptions
  "aria-label": string; // Accessible name
  "aria-labelledby": string; // ID of labelling element
  "aria-describedby": string; // ID of describing element

  // States
  "aria-expanded": boolean; // Expandable element state
  "aria-selected": boolean; // Selection state
  "aria-checked": boolean | "mixed"; // Checkbox/switch state
  "aria-pressed": boolean | "mixed"; // Toggle button state
  "aria-disabled": boolean; // Disabled state
  "aria-hidden": boolean; // Hidden from assistive tech

  // Live regions
  "aria-live": "off" | "polite" | "assertive";
  "aria-atomic": boolean;
  "aria-relevant": string;

  // Relationships
  "aria-controls": string; // ID of controlled element
  "aria-owns": string; // ID of owned elements
  "aria-haspopup": boolean | "menu" | "dialog";

  // Other
  "aria-current": "page" | "step" | "location" | "date" | "time" | boolean;
  "aria-invalid": boolean | "grammar" | "spelling";
  "aria-required": boolean;
}
```

#### ARIA Examples

```tsx
// Accessible modal dialog
function Modal({ isOpen, onClose, title, children }) {
  const titleId = useId();

  useEffect(() => {
    if (isOpen) {
      // Trap focus inside modal
      const focusableElements = modalRef.current.querySelectorAll(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
      );
      const firstElement = focusableElements[0];
      const lastElement = focusableElements[focusableElements.length - 1];

      firstElement?.focus();

      const handleTab = (e: KeyboardEvent) => {
        if (e.key === "Tab") {
          if (e.shiftKey && document.activeElement === firstElement) {
            e.preventDefault();
            lastElement?.focus();
          } else if (!e.shiftKey && document.activeElement === lastElement) {
            e.preventDefault();
            firstElement?.focus();
          }
        }
        if (e.key === "Escape") {
          onClose();
        }
      };

      document.addEventListener("keydown", handleTab);
      return () => document.removeEventListener("keydown", handleTab);
    }
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      ref={modalRef}
    >
      <h2 id={titleId}>{title}</h2>
      {children}
      <button onClick={onClose} aria-label="Close dialog">
        Close
      </button>
    </div>
  );
}

// Accessible tabs
function Tabs({ tabs }) {
  const [activeIndex, setActiveIndex] = useState(0);

  const handleKeyDown = (e: KeyboardEvent, index: number) => {
    let newIndex = index;

    switch (e.key) {
      case "ArrowRight":
        newIndex = (index + 1) % tabs.length;
        break;
      case "ArrowLeft":
        newIndex = (index - 1 + tabs.length) % tabs.length;
        break;
      case "Home":
        newIndex = 0;
        break;
      case "End":
        newIndex = tabs.length - 1;
        break;
      default:
        return;
    }

    e.preventDefault();
    setActiveIndex(newIndex);
    document.getElementById(`tab-${newIndex}`)?.focus();
  };

  return (
    <div>
      <div role="tablist" aria-label="Content tabs">
        {tabs.map((tab, index) => (
          <button
            key={index}
            id={`tab-${index}`}
            role="tab"
            aria-selected={activeIndex === index}
            aria-controls={`panel-${index}`}
            tabIndex={activeIndex === index ? 0 : -1}
            onClick={() => setActiveIndex(index)}
            onKeyDown={(e) => handleKeyDown(e, index)}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {tabs.map((tab, index) => (
        <div
          key={index}
          id={`panel-${index}`}
          role="tabpanel"
          aria-labelledby={`tab-${index}`}
          hidden={activeIndex !== index}
          tabIndex={0}
        >
          {tab.content}
        </div>
      ))}
    </div>
  );
}

// Live region for dynamic updates
function Notification({ message, type }) {
  return (
    <div
      role="alert"
      aria-live={type === "error" ? "assertive" : "polite"}
      aria-atomic="true"
    >
      {message}
    </div>
  );
}
```

### 4. Keyboard Navigation

```tsx
// Keyboard navigation utilities

// Focus management hook
function useFocusManagement() {
  const focusableSelector = [
    "a[href]",
    "button:not([disabled])",
    "input:not([disabled])",
    "select:not([disabled])",
    "textarea:not([disabled])",
    '[tabindex]:not([tabindex="-1"])',
  ].join(", ");

  const getFocusableElements = (container: HTMLElement) => {
    return Array.from(container.querySelectorAll(focusableSelector));
  };

  const trapFocus = (container: HTMLElement) => {
    const elements = getFocusableElements(container);
    const first = elements[0] as HTMLElement;
    const last = elements[elements.length - 1] as HTMLElement;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;

      if (e.shiftKey) {
        if (document.activeElement === first) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };

    container.addEventListener("keydown", handleKeyDown);
    return () => container.removeEventListener("keydown", handleKeyDown);
  };

  return { getFocusableElements, trapFocus };
}

// Roving tabindex for grouped elements
function useRovingTabindex<T extends HTMLElement>(
  items: T[],
  options: { orientation: "horizontal" | "vertical" | "both" } = {
    orientation: "horizontal",
  },
) {
  const [focusedIndex, setFocusedIndex] = useState(0);

  const handleKeyDown = (e: KeyboardEvent, index: number) => {
    const { orientation } = options;
    let newIndex = index;

    const prevKeys = orientation === "vertical" ? ["ArrowUp"] : ["ArrowLeft"];
    const nextKeys =
      orientation === "vertical" ? ["ArrowDown"] : ["ArrowRight"];

    if (orientation === "both") {
      prevKeys.push("ArrowUp", "ArrowLeft");
      nextKeys.push("ArrowDown", "ArrowRight");
    }

    if (prevKeys.includes(e.key)) {
      newIndex = (index - 1 + items.length) % items.length;
    } else if (nextKeys.includes(e.key)) {
      newIndex = (index + 1) % items.length;
    } else if (e.key === "Home") {
      newIndex = 0;
    } else if (e.key === "End") {
      newIndex = items.length - 1;
    } else {
      return;
    }

    e.preventDefault();
    setFocusedIndex(newIndex);
    items[newIndex]?.focus();
  };

  return {
    focusedIndex,
    getTabIndex: (index: number) => (index === focusedIndex ? 0 : -1),
    handleKeyDown,
  };
}

// Skip link component
function SkipLink({ targetId, children = "Skip to main content" }) {
  return (
    <a
      href={`#${targetId}`}
      className="skip-link"
      style={{
        position: "absolute",
        left: "-9999px",
        top: "auto",
        width: "1px",
        height: "1px",
        overflow: "hidden",
      }}
      onFocus={(e) => {
        e.currentTarget.style.left = "0";
        e.currentTarget.style.width = "auto";
        e.currentTarget.style.height = "auto";
      }}
      onBlur={(e) => {
        e.currentTarget.style.left = "-9999px";
        e.currentTarget.style.width = "1px";
        e.currentTarget.style.height = "1px";
      }}
    >
      {children}
    </a>
  );
}
```

### 5. Screen Reader Testing

```typescript
// Screen reader testing guide

const screenReaderTesting = {
  // Common screen readers
  readers: {
    nvda: {
      platform: "Windows",
      cost: "Free",
      shortcuts: {
        startReading: "NVDA + Down Arrow",
        stopReading: "Ctrl",
        nextHeading: "H",
        nextLink: "K",
        nextFormField: "F",
        nextLandmark: "D",
        elementsList: "NVDA + F7",
      },
    },
    jaws: {
      platform: "Windows",
      cost: "Paid",
      shortcuts: {
        startReading: "Insert + Down Arrow",
        stopReading: "Ctrl",
        nextHeading: "H",
        nextLink: "Tab",
        headingsList: "Insert + F6",
      },
    },
    voiceover: {
      platform: "macOS/iOS",
      cost: "Built-in",
      shortcuts: {
        toggle: "Cmd + F5",
        nextElement: "VO + Right Arrow",
        previousElement: "VO + Left Arrow",
        rotor: "VO + U",
        activate: "VO + Space",
      },
    },
    talkback: {
      platform: "Android",
      cost: "Built-in",
      gestures: {
        nextElement: "Swipe Right",
        previousElement: "Swipe Left",
        activate: "Double Tap",
        scrollForward: "Two finger swipe up",
      },
    },
  },

  // Testing checklist
  checklist: [
    "All images have appropriate alt text",
    "Form labels are properly associated with inputs",
    "Headings create a logical outline",
    'Links have descriptive text (not "click here")',
    "Dynamic content updates are announced",
    "Focus is managed correctly in modals",
    "Error messages are associated with form fields",
    "Tables have proper headers and captions",
  ],
};

// Announce utility for screen readers
function announce(
  message: string,
  priority: "polite" | "assertive" = "polite",
) {
  const announcer = document.createElement("div");
  announcer.setAttribute("aria-live", priority);
  announcer.setAttribute("aria-atomic", "true");
  announcer.setAttribute("class", "sr-only");
  announcer.style.cssText = `
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  `;

  document.body.appendChild(announcer);

  // Delay to ensure screen reader picks up the change
  setTimeout(() => {
    announcer.textContent = message;
  }, 100);

  // Clean up
  setTimeout(() => {
    document.body.removeChild(announcer);
  }, 1000);
}

// Usage
announce("Form submitted successfully");
announce("Error: Please fill in all required fields", "assertive");
```

### 6. Color Contrast and Visual Accessibility

```typescript
// Color contrast utilities

function getLuminance(r: number, g: number, b: number): number {
  const [rs, gs, bs] = [r, g, b].map((c) => {
    c = c / 255;
    return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
}

function getContrastRatio(color1: string, color2: string): number {
  const rgb1 = hexToRgb(color1);
  const rgb2 = hexToRgb(color2);

  const l1 = getLuminance(rgb1.r, rgb1.g, rgb1.b);
  const l2 = getLuminance(rgb2.r, rgb2.g, rgb2.b);

  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);

  return (lighter + 0.05) / (darker + 0.05);
}

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  return result
    ? {
        r: parseInt(result[1], 16),
        g: parseInt(result[2], 16),
        b: parseInt(result[3], 16),
      }
    : { r: 0, g: 0, b: 0 };
}

function meetsWCAGContrast(
  foreground: string,
  background: string,
  level: "AA" | "AAA" = "AA",
  isLargeText: boolean = false,
): boolean {
  const ratio = getContrastRatio(foreground, background);

  const requirements = {
    AA: { normal: 4.5, large: 3 },
    AAA: { normal: 7, large: 4.5 },
  };

  const required = requirements[level][isLargeText ? "large" : "normal"];
  return ratio >= required;
}

// Usage
const passes = meetsWCAGContrast("#333333", "#ffffff", "AA");
console.log(
  `Contrast ratio: ${getContrastRatio("#333333", "#ffffff").toFixed(2)}:1`,
);

// Accessible color palette generator
function generateAccessiblePalette(
  baseColor: string,
  background: string = "#ffffff",
) {
  const shades = [];
  const rgb = hexToRgb(baseColor);

  for (let i = 0; i <= 100; i += 10) {
    const factor = i / 100;
    const shade = {
      r: Math.round(rgb.r * factor),
      g: Math.round(rgb.g * factor),
      b: Math.round(rgb.b * factor),
    };
    const hex = `#${shade.r.toString(16).padStart(2, "0")}${shade.g.toString(16).padStart(2, "0")}${shade.b.toString(16).padStart(2, "0")}`;
    const ratio = getContrastRatio(hex, background);

    shades.push({
      shade: i,
      hex,
      contrastRatio: ratio.toFixed(2),
      passesAA: ratio >= 4.5,
      passesAAA: ratio >= 7,
    });
  }

  return shades;
}

// Focus visible styles
const focusStyles = `
  /* Remove default outline */
  :focus {
    outline: none;
  }

  /* Add visible focus for keyboard users */
  :focus-visible {
    outline: 2px solid #005fcc;
    outline-offset: 2px;
  }

  /* High contrast mode support */
  @media (prefers-contrast: high) {
    :focus-visible {
      outline: 3px solid currentColor;
      outline-offset: 3px;
    }
  }

  /* Reduced motion support */
  @media (prefers-reduced-motion: reduce) {
    *,
    *::before,
    *::after {
      animation-duration: 0.01ms !important;
      animation-iteration-count: 1 !important;
      transition-duration: 0.01ms !important;
    }
  }
`;
```

### 7. Automated A11y Testing Tools

```typescript
// Automated accessibility testing setup

// Jest + Testing Library
import { render, screen } from '@testing-library/react';
import { axe, toHaveNoViolations } from 'jest-axe';

expect.extend(toHaveNoViolations);

describe('Accessibility Tests', () => {
  it('should have no accessibility violations', async () => {
    const { container } = render(<MyComponent />);
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should have accessible form', () => {
    render(<LoginForm />);

    // Check for proper labels
    expect(screen.getByLabelText('Email')).toBeInTheDocument();
    expect(screen.getByLabelText('Password')).toBeInTheDocument();

    // Check for proper roles
    expect(screen.getByRole('form')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sign In' })).toBeInTheDocument();
  });

  it('should manage focus correctly', () => {
    render(<Modal isOpen={true} />);

    // First focusable element should be focused
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Close' }));
  });
});

// Playwright accessibility testing
import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.describe('Accessibility', () => {
  test('homepage should not have accessibility issues', async ({ page }) => {
    await page.goto('/');

    const accessibilityScanResults = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa', 'wcag21aa'])
      .analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });

  test('should be navigable by keyboard', async ({ page }) => {
    await page.goto('/');

    // Tab to skip link
    await page.keyboard.press('Tab');
    await expect(page.getByText('Skip to main content')).toBeFocused();

    // Tab to navigation
    await page.keyboard.press('Tab');
    await expect(page.getByRole('link', { name: 'Home' })).toBeFocused();
  });
});

// CI/CD integration for accessibility
const accessibilityConfig = {
  // Lighthouse CI config
  lighthouse: {
    assertions: {
      'categories:accessibility': ['error', { minScore: 0.9 }],
      'color-contrast': 'error',
      'document-title': 'error',
      'html-has-lang': 'error',
      'image-alt': 'error',
      'label': 'error',
      'link-name': 'error',
      'meta-viewport': 'error',
    },
  },

  // Pa11y CI config
  pa11y: {
    standard: 'WCAG2AA',
    runners: ['axe', 'htmlcs'],
    ignore: [
      'WCAG2AA.Principle1.Guideline1_4.1_4_3.G18.Fail', // Ignore specific rules if needed
    ],
  },

  // Axe configuration
  axe: {
    runOnly: {
      type: 'tag',
      values: ['wcag2a', 'wcag2aa', 'wcag21aa', 'best-practice'],
    },
    rules: {
      'color-contrast': { enabled: true },
      'valid-lang': { enabled: true },
    },
  },
};
```

## Best Practices

1. **Start with Semantic HTML**: Proper HTML elements provide built-in accessibility.

2. **Test with Real Users**: Include users with disabilities in testing when possible.

3. **Use ARIA Sparingly**: First rule of ARIA is "don't use ARIA" if HTML can do it.

4. **Maintain Focus Management**: Ensure logical focus order and visible focus indicators.

5. **Provide Text Alternatives**: All non-text content needs text alternatives.

6. **Ensure Keyboard Accessibility**: All functionality must work with keyboard only.

7. **Test with Multiple Tools**: Use both automated tools and manual testing.

8. **Document Accessibility Features**: Help users discover accessibility options.

## Examples

### Complete Accessible Form

```tsx
function AccessibleForm() {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const errorSummaryRef = useRef<HTMLDivElement>(null);

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    const newErrors: Record<string, string> = {};

    // Validation
    if (!formData.email) {
      newErrors.email = "Email is required";
    }
    if (!formData.password) {
      newErrors.password = "Password is required";
    }

    if (Object.keys(newErrors).length > 0) {
      setErrors(newErrors);
      // Focus error summary for screen readers
      errorSummaryRef.current?.focus();
      announce(
        "Form has errors. Please correct them and try again.",
        "assertive",
      );
    } else {
      // Submit form
      announce("Form submitted successfully", "polite");
    }
  };

  return (
    <form onSubmit={handleSubmit} aria-labelledby="form-title" noValidate>
      <h1 id="form-title">Sign Up</h1>

      {Object.keys(errors).length > 0 && (
        <div
          ref={errorSummaryRef}
          role="alert"
          aria-labelledby="error-summary-title"
          tabIndex={-1}
          className="error-summary"
        >
          <h2 id="error-summary-title">There are errors in the form</h2>
          <ul>
            {Object.entries(errors).map(([field, message]) => (
              <li key={field}>
                <a href={`#${field}`}>{message}</a>
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="form-field">
        <label htmlFor="email">
          Email address
          <span aria-hidden="true">*</span>
          <span className="sr-only">(required)</span>
        </label>
        <input
          type="email"
          id="email"
          name="email"
          autoComplete="email"
          aria-required="true"
          aria-invalid={!!errors.email}
          aria-describedby={errors.email ? "email-error" : undefined}
        />
        {errors.email && (
          <span id="email-error" role="alert" className="error">
            {errors.email}
          </span>
        )}
      </div>

      <div className="form-field">
        <label htmlFor="password">
          Password
          <span aria-hidden="true">*</span>
          <span className="sr-only">(required)</span>
        </label>
        <input
          type="password"
          id="password"
          name="password"
          autoComplete="new-password"
          aria-required="true"
          aria-invalid={!!errors.password}
          aria-describedby="password-hint password-error"
        />
        <span id="password-hint" className="hint">
          Must be at least 8 characters
        </span>
        {errors.password && (
          <span id="password-error" role="alert" className="error">
            {errors.password}
          </span>
        )}
      </div>

      <button type="submit">Create Account</button>
    </form>
  );
}
```
