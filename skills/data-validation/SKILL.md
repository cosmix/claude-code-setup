---
name: data-validation
description: Data validation patterns including schema validation, input sanitization, output encoding, and type coercion. Use when implementing form validation, API input validation, JSON Schema, Zod, Pydantic, sanitization, XSS prevention, or custom validators.
---

# Data Validation

## Overview

Data validation ensures that input data meets expected formats, types, and constraints before processing. This skill covers schema validation libraries, input sanitization, output encoding, type coercion strategies, and comprehensive error handling for validation failures.

## Key Concepts

### JSON Schema Validation

```typescript
import Ajv, { JSONSchemaType, ValidateFunction } from "ajv";
import addFormats from "ajv-formats";

// Initialize Ajv with formats
const ajv = new Ajv({
  allErrors: true, // Return all errors, not just first
  removeAdditional: true, // Remove properties not in schema
  useDefaults: true, // Apply default values
  coerceTypes: true, // Coerce types when possible
});
addFormats(ajv);

// Define schema with TypeScript type
interface CreateUserRequest {
  email: string;
  password: string;
  name: string;
  age?: number;
  role: "user" | "admin" | "moderator";
  preferences?: {
    newsletter: boolean;
    theme: "light" | "dark";
  };
}

const createUserSchema: JSONSchemaType<CreateUserRequest> = {
  type: "object",
  properties: {
    email: { type: "string", format: "email", maxLength: 255 },
    password: {
      type: "string",
      minLength: 12,
      maxLength: 128,
      pattern:
        "^(?=.*[a-z])(?=.*[A-Z])(?=.*\\d)(?=.*[@$!%*?&])[A-Za-z\\d@$!%*?&]+$",
    },
    name: { type: "string", minLength: 1, maxLength: 100 },
    age: { type: "integer", minimum: 13, maximum: 150, nullable: true },
    role: { type: "string", enum: ["user", "admin", "moderator"] },
    preferences: {
      type: "object",
      properties: {
        newsletter: { type: "boolean", default: false },
        theme: { type: "string", enum: ["light", "dark"], default: "light" },
      },
      required: ["newsletter", "theme"],
      additionalProperties: false,
      nullable: true,
    },
  },
  required: ["email", "password", "name", "role"],
  additionalProperties: false,
};

// Compile and cache validator
const validateCreateUser = ajv.compile(createUserSchema);

// Usage with error formatting
function validate<T>(
  validator: ValidateFunction<T>,
  data: unknown,
): { success: true; data: T } | { success: false; errors: ValidationError[] } {
  if (validator(data)) {
    return { success: true, data };
  }

  const errors: ValidationError[] = (validator.errors || []).map((err) => ({
    field:
      err.instancePath.replace(/^\//, "").replace(/\//g, ".") ||
      err.params.missingProperty,
    message: formatAjvError(err),
    code: err.keyword,
  }));

  return { success: false, errors };
}

function formatAjvError(error: Ajv.ErrorObject): string {
  switch (error.keyword) {
    case "required":
      return `${error.params.missingProperty} is required`;
    case "minLength":
      return `Must be at least ${error.params.limit} characters`;
    case "maxLength":
      return `Must be at most ${error.params.limit} characters`;
    case "format":
      return `Invalid ${error.params.format} format`;
    case "enum":
      return `Must be one of: ${error.params.allowedValues.join(", ")}`;
    case "pattern":
      return "Invalid format";
    case "minimum":
      return `Must be at least ${error.params.limit}`;
    case "maximum":
      return `Must be at most ${error.params.limit}`;
    default:
      return error.message || "Invalid value";
  }
}
```

### Zod Validation (TypeScript)

```typescript
import { z, ZodError, ZodSchema } from "zod";

// Basic schemas
const emailSchema = z.string().email().max(255);
const passwordSchema = z
  .string()
  .min(12, "Password must be at least 12 characters")
  .max(128)
  .regex(/[a-z]/, "Password must contain a lowercase letter")
  .regex(/[A-Z]/, "Password must contain an uppercase letter")
  .regex(/[0-9]/, "Password must contain a number")
  .regex(/[^a-zA-Z0-9]/, "Password must contain a special character");

// Complex schema with transforms and refinements
const createUserSchema = z
  .object({
    email: emailSchema.transform((e) => e.toLowerCase().trim()),
    password: passwordSchema,
    confirmPassword: z.string(),
    name: z
      .string()
      .min(1)
      .max(100)
      .transform((n) => n.trim()),
    age: z.number().int().min(13).max(150).optional(),
    role: z.enum(["user", "admin", "moderator"]).default("user"),
    tags: z.array(z.string().max(50)).max(10).default([]),
    metadata: z.record(z.string(), z.unknown()).optional(),
    preferences: z
      .object({
        newsletter: z.boolean().default(false),
        theme: z.enum(["light", "dark"]).default("light"),
        notifications: z
          .object({
            email: z.boolean().default(true),
            push: z.boolean().default(false),
            sms: z.boolean().default(false),
          })
          .default({}),
      })
      .default({}),
  })
  .refine((data) => data.password === data.confirmPassword, {
    message: "Passwords do not match",
    path: ["confirmPassword"],
  })
  .transform(({ confirmPassword, ...data }) => data); // Remove confirmPassword

// Infer TypeScript types from schema
type CreateUserInput = z.input<typeof createUserSchema>;
type CreateUserOutput = z.output<typeof createUserSchema>;

// Validation helper with formatted errors
interface ValidationResult<T> {
  success: boolean;
  data?: T;
  errors?: Array<{
    field: string;
    message: string;
  }>;
}

function validateWithZod<T>(
  schema: ZodSchema<T>,
  data: unknown,
): ValidationResult<T> {
  const result = schema.safeParse(data);

  if (result.success) {
    return { success: true, data: result.data };
  }

  const errors = result.error.errors.map((err) => ({
    field: err.path.join("."),
    message: err.message,
  }));

  return { success: false, errors };
}

// Custom refinements
const uniqueEmailSchema = emailSchema.refine(
  async (email) => {
    const exists = await db.users.findByEmail(email);
    return !exists;
  },
  { message: "Email already registered" },
);

// Conditional validation
const formSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("individual"),
    firstName: z.string().min(1),
    lastName: z.string().min(1),
    ssn: z.string().regex(/^\d{3}-\d{2}-\d{4}$/),
  }),
  z.object({
    type: z.literal("business"),
    companyName: z.string().min(1),
    ein: z.string().regex(/^\d{2}-\d{7}$/),
  }),
]);

// Recursive schemas
interface Category {
  name: string;
  children?: Category[];
}

const categorySchema: z.ZodType<Category> = z.lazy(() =>
  z.object({
    name: z.string().min(1),
    children: z.array(categorySchema).optional(),
  }),
);
```

### Pydantic Validation (Python)

```python
from datetime import datetime
from typing import Optional, List, Literal
from pydantic import (
    BaseModel,
    Field,
    EmailStr,
    validator,
    root_validator,
    constr,
    conint,
)
import re

# Basic model with field validation
class CreateUserRequest(BaseModel):
    email: EmailStr
    password: constr(min_length=12, max_length=128)
    name: constr(min_length=1, max_length=100)
    age: Optional[conint(ge=13, le=150)] = None
    role: Literal['user', 'admin', 'moderator'] = 'user'
    tags: List[str] = Field(default_factory=list, max_items=10)

    class Config:
        # Strip whitespace from strings
        anystr_strip_whitespace = True
        # Validate on assignment
        validate_assignment = True
        # Use enum values
        use_enum_values = True

    @validator('email')
    def email_lowercase(cls, v):
        return v.lower()

    @validator('password')
    def password_strength(cls, v):
        if not re.search(r'[a-z]', v):
            raise ValueError('Password must contain a lowercase letter')
        if not re.search(r'[A-Z]', v):
            raise ValueError('Password must contain an uppercase letter')
        if not re.search(r'\d', v):
            raise ValueError('Password must contain a number')
        if not re.search(r'[^a-zA-Z0-9]', v):
            raise ValueError('Password must contain a special character')
        return v

    @validator('tags', each_item=True)
    def validate_tag(cls, v):
        if len(v) > 50:
            raise ValueError('Tag must be at most 50 characters')
        return v.strip().lower()

# Nested models
class Address(BaseModel):
    street: str
    city: str
    state: constr(min_length=2, max_length=2)
    zip_code: constr(regex=r'^\d{5}(-\d{4})?$')
    country: str = 'US'

class UserProfile(BaseModel):
    user: CreateUserRequest
    addresses: List[Address] = Field(default_factory=list, max_items=5)
    primary_address_index: int = 0

    @root_validator
    def validate_primary_address(cls, values):
        addresses = values.get('addresses', [])
        primary_index = values.get('primary_address_index', 0)

        if addresses and primary_index >= len(addresses):
            raise ValueError('Primary address index out of range')

        return values

# Generic response model
from typing import TypeVar, Generic

T = TypeVar('T')

class ApiResponse(BaseModel, Generic[T]):
    success: bool
    data: Optional[T] = None
    errors: Optional[List[dict]] = None
    timestamp: datetime = Field(default_factory=datetime.utcnow)

# Custom validator with database lookup
from pydantic import validator
import asyncio

class UniqueEmailModel(BaseModel):
    email: EmailStr

    @validator('email')
    def email_must_be_unique(cls, v):
        # Note: This is synchronous; use root_validator for async
        from app.db import user_exists_sync
        if user_exists_sync(v):
            raise ValueError('Email already registered')
        return v

# Validation error handling
from pydantic import ValidationError
from fastapi import HTTPException

def validate_request(model_class, data: dict):
    try:
        return model_class(**data)
    except ValidationError as e:
        errors = []
        for error in e.errors():
            errors.append({
                'field': '.'.join(str(loc) for loc in error['loc']),
                'message': error['msg'],
                'type': error['type'],
            })
        raise HTTPException(status_code=422, detail={'errors': errors})
```

### Input Sanitization

```typescript
import DOMPurify from "dompurify";
import { JSDOM } from "jsdom";
import validator from "validator";

// Server-side DOMPurify setup
const window = new JSDOM("").window;
const purify = DOMPurify(window);

// HTML sanitization
function sanitizeHtml(dirty: string, options?: DOMPurify.Config): string {
  const defaultOptions: DOMPurify.Config = {
    ALLOWED_TAGS: ["b", "i", "em", "strong", "a", "p", "br", "ul", "ol", "li"],
    ALLOWED_ATTR: ["href", "target", "rel"],
    ALLOW_DATA_ATTR: false,
    ADD_ATTR: ["target"], // Add target="_blank" to links
    FORBID_TAGS: ["script", "style", "iframe", "form", "input"],
    FORBID_ATTR: ["onerror", "onclick", "onload"],
  };

  return purify.sanitize(dirty, { ...defaultOptions, ...options });
}

// Rich text sanitization (more permissive)
function sanitizeRichText(dirty: string): string {
  return purify.sanitize(dirty, {
    ALLOWED_TAGS: [
      "h1",
      "h2",
      "h3",
      "h4",
      "h5",
      "h6",
      "p",
      "br",
      "hr",
      "b",
      "i",
      "em",
      "strong",
      "u",
      "s",
      "strike",
      "ul",
      "ol",
      "li",
      "a",
      "img",
      "blockquote",
      "pre",
      "code",
      "table",
      "thead",
      "tbody",
      "tr",
      "th",
      "td",
    ],
    ALLOWED_ATTR: ["href", "src", "alt", "title", "class", "id"],
    ALLOW_DATA_ATTR: false,
  });
}

// SQL-safe string (use parameterized queries instead when possible)
function sanitizeForSql(input: string): string {
  return input
    .replace(/'/g, "''")
    .replace(/\\/g, "\\\\")
    .replace(/\x00/g, "\\0")
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\x1a/g, "\\Z");
}

// Filename sanitization
function sanitizeFilename(filename: string): string {
  return filename
    .replace(/[^a-zA-Z0-9._-]/g, "_") // Replace special chars
    .replace(/\.{2,}/g, ".") // Remove consecutive dots
    .replace(/^\.+|\.+$/g, "") // Remove leading/trailing dots
    .substring(0, 255); // Limit length
}

// Path traversal prevention
function sanitizePath(userPath: string, basePath: string): string {
  const path = require("path");
  const resolvedPath = path.resolve(basePath, userPath);

  if (!resolvedPath.startsWith(path.resolve(basePath))) {
    throw new Error("Path traversal detected");
  }

  return resolvedPath;
}

// Comprehensive input sanitizer
interface SanitizationOptions {
  trim?: boolean;
  lowercase?: boolean;
  stripHtml?: boolean;
  maxLength?: number;
  allowedChars?: RegExp;
}

function sanitizeString(
  input: string,
  options: SanitizationOptions = {},
): string {
  let result = input;

  if (options.trim !== false) {
    result = result.trim();
  }

  if (options.stripHtml) {
    result = validator.stripLow(validator.escape(result));
  }

  if (options.lowercase) {
    result = result.toLowerCase();
  }

  if (options.allowedChars) {
    result = result.replace(
      new RegExp(`[^${options.allowedChars.source}]`, "g"),
      "",
    );
  }

  if (options.maxLength) {
    result = result.substring(0, options.maxLength);
  }

  // Remove null bytes
  result = result.replace(/\x00/g, "");

  return result;
}

// Common sanitization presets
const sanitizers = {
  username: (input: string) =>
    sanitizeString(input, {
      lowercase: true,
      maxLength: 30,
      allowedChars: /[a-z0-9_-]/,
    }),

  email: (input: string) => validator.normalizeEmail(input) || "",

  phone: (input: string) => input.replace(/[^0-9+()-\s]/g, "").substring(0, 20),

  slug: (input: string) =>
    sanitizeString(input, {
      lowercase: true,
      maxLength: 100,
    })
      .replace(/\s+/g, "-")
      .replace(/[^a-z0-9-]/g, ""),

  searchQuery: (input: string) =>
    sanitizeString(input, {
      trim: true,
      maxLength: 200,
      stripHtml: true,
    }),
};
```

### Output Encoding

```typescript
// HTML encoding
function encodeHtml(str: string): string {
  const entities: Record<string, string> = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#x27;",
    "/": "&#x2F;",
    "`": "&#x60;",
    "=": "&#x3D;",
  };

  return str.replace(/[&<>"'`=/]/g, (char) => entities[char]);
}

// JavaScript string encoding (for embedding in <script> tags)
function encodeJsString(str: string): string {
  return str
    .replace(/\\/g, "\\\\")
    .replace(/'/g, "\\'")
    .replace(/"/g, '\\"')
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\t/g, "\\t")
    .replace(/</g, "\\x3c")
    .replace(/>/g, "\\x3e")
    .replace(/&/g, "\\x26");
}

// URL encoding
function encodeUrlParam(str: string): string {
  return encodeURIComponent(str);
}

// CSS encoding
function encodeCss(str: string): string {
  return str.replace(/[^a-zA-Z0-9]/g, (char) => {
    const hex = char.charCodeAt(0).toString(16);
    return `\\${hex} `;
  });
}

// JSON encoding (safe for embedding in HTML)
function encodeJsonForHtml(obj: unknown): string {
  return JSON.stringify(obj)
    .replace(/</g, "\\u003c")
    .replace(/>/g, "\\u003e")
    .replace(/&/g, "\\u0026")
    .replace(/'/g, "\\u0027");
}

// Context-aware output encoding
type OutputContext = "html" | "htmlAttribute" | "javascript" | "url" | "css";

function encode(str: string, context: OutputContext): string {
  switch (context) {
    case "html":
      return encodeHtml(str);
    case "htmlAttribute":
      return encodeHtml(str).replace(/"/g, "&quot;");
    case "javascript":
      return encodeJsString(str);
    case "url":
      return encodeUrlParam(str);
    case "css":
      return encodeCss(str);
    default:
      return encodeHtml(str);
  }
}

// React-style escaping (for JSX)
function escapeForReact(str: string): string {
  // React already escapes, but for dangerouslySetInnerHTML:
  return encodeHtml(str);
}

// Template literal tag for safe HTML
function safeHtml(strings: TemplateStringsArray, ...values: unknown[]): string {
  return strings.reduce((result, str, i) => {
    const value = values[i - 1];
    const encoded =
      typeof value === "string" ? encodeHtml(value) : String(value ?? "");
    return result + encoded + str;
  });
}

// Usage
const userInput = '<script>alert("xss")</script>';
const safe = safeHtml`<div class="user-content">${userInput}</div>`;
// Result: <div class="user-content">&lt;script&gt;alert(&quot;xss&quot;)&lt;/script&gt;</div>
```

### Type Coercion Strategies

```typescript
// Safe type coercion utilities
const coerce = {
  toString(value: unknown, defaultValue: string = ""): string {
    if (value === null || value === undefined) return defaultValue;
    if (typeof value === "string") return value;
    if (typeof value === "number" && !isNaN(value)) return String(value);
    if (typeof value === "boolean") return String(value);
    return defaultValue;
  },

  toNumber(value: unknown, defaultValue: number = 0): number {
    if (typeof value === "number" && !isNaN(value)) return value;
    if (typeof value === "string") {
      const parsed = parseFloat(value);
      if (!isNaN(parsed)) return parsed;
    }
    return defaultValue;
  },

  toInt(value: unknown, defaultValue: number = 0): number {
    const num = coerce.toNumber(value, NaN);
    if (isNaN(num)) return defaultValue;
    return Math.trunc(num);
  },

  toBoolean(value: unknown, defaultValue: boolean = false): boolean {
    if (typeof value === "boolean") return value;
    if (typeof value === "string") {
      const lower = value.toLowerCase().trim();
      if (["true", "1", "yes", "on"].includes(lower)) return true;
      if (["false", "0", "no", "off"].includes(lower)) return false;
    }
    if (typeof value === "number") return value !== 0;
    return defaultValue;
  },

  toDate(value: unknown, defaultValue: Date | null = null): Date | null {
    if (value instanceof Date && !isNaN(value.getTime())) return value;
    if (typeof value === "string" || typeof value === "number") {
      const date = new Date(value);
      if (!isNaN(date.getTime())) return date;
    }
    return defaultValue;
  },

  toArray<T>(value: unknown, itemCoercer?: (item: unknown) => T): T[] {
    if (Array.isArray(value)) {
      return itemCoercer ? value.map(itemCoercer) : (value as T[]);
    }
    if (value === null || value === undefined) return [];
    return itemCoercer ? [itemCoercer(value)] : [value as T];
  },

  toEnum<T extends string>(
    value: unknown,
    allowedValues: readonly T[],
    defaultValue: T,
  ): T {
    const str = coerce.toString(value);
    if (allowedValues.includes(str as T)) return str as T;
    return defaultValue;
  },
};

// Query parameter coercion
interface QueryParams {
  page: number;
  limit: number;
  sort: "asc" | "desc";
  filter: string;
  active: boolean;
  tags: string[];
}

function parseQueryParams(query: Record<string, unknown>): QueryParams {
  return {
    page: Math.max(1, coerce.toInt(query.page, 1)),
    limit: Math.min(100, Math.max(1, coerce.toInt(query.limit, 20))),
    sort: coerce.toEnum(query.sort, ["asc", "desc"] as const, "desc"),
    filter: coerce.toString(query.filter).substring(0, 200),
    active: coerce.toBoolean(query.active, true),
    tags: coerce.toArray(query.tags, coerce.toString).slice(0, 10),
  };
}

// Form data coercion with validation
interface FormDataCoercer<T> {
  coerce: (value: unknown) => T;
  validate?: (value: T) => boolean;
  errorMessage?: string;
}

function coerceFormData<T extends Record<string, unknown>>(
  data: Record<string, unknown>,
  schema: { [K in keyof T]: FormDataCoercer<T[K]> },
):
  | { success: true; data: T }
  | { success: false; errors: Record<string, string> } {
  const result: Partial<T> = {};
  const errors: Record<string, string> = {};

  for (const [key, coercer] of Object.entries(schema)) {
    const value = data[key];
    const coerced = coercer.coerce(value);

    if (coercer.validate && !coercer.validate(coerced)) {
      errors[key] = coercer.errorMessage || `Invalid value for ${key}`;
    } else {
      result[key as keyof T] = coerced as T[keyof T];
    }
  }

  if (Object.keys(errors).length > 0) {
    return { success: false, errors };
  }

  return { success: true, data: result as T };
}
```

### Custom Validators

```typescript
// Validator builder pattern
type ValidatorFn<T> = (value: T) => boolean | string;

class Validator<T> {
  private validators: Array<{ fn: ValidatorFn<T>; message: string }> = [];

  add(fn: ValidatorFn<T>, message: string): this {
    this.validators.push({ fn, message });
    return this;
  }

  validate(value: T): { valid: boolean; errors: string[] } {
    const errors: string[] = [];

    for (const { fn, message } of this.validators) {
      const result = fn(value);
      if (result === false) {
        errors.push(message);
      } else if (typeof result === "string") {
        errors.push(result);
      }
    }

    return { valid: errors.length === 0, errors };
  }
}

// Common validators
const validators = {
  required:
    (message = "This field is required") =>
    (value: unknown) =>
      value !== null && value !== undefined && value !== "" ? true : message,

  minLength: (min: number, message?: string) => (value: string) =>
    value.length >= min
      ? true
      : message || `Must be at least ${min} characters`,

  maxLength: (max: number, message?: string) => (value: string) =>
    value.length <= max ? true : message || `Must be at most ${max} characters`,

  pattern: (regex: RegExp, message: string) => (value: string) =>
    regex.test(value) ? true : message,

  email:
    (message = "Invalid email address") =>
    (value: string) =>
      /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value) ? true : message,

  url:
    (message = "Invalid URL") =>
    (value: string) => {
      try {
        new URL(value);
        return true;
      } catch {
        return message;
      }
    },

  range: (min: number, max: number, message?: string) => (value: number) =>
    value >= min && value <= max
      ? true
      : message || `Must be between ${min} and ${max}`,

  integer:
    (message = "Must be a whole number") =>
    (value: number) =>
      Number.isInteger(value) ? true : message,

  positive:
    (message = "Must be a positive number") =>
    (value: number) =>
      value > 0 ? true : message,

  oneOf:
    <T>(allowed: T[], message?: string) =>
    (value: T) =>
      allowed.includes(value)
        ? true
        : message || `Must be one of: ${allowed.join(", ")}`,

  custom:
    <T>(fn: (value: T) => boolean, message: string) =>
    (value: T) =>
      fn(value) ? true : message,
};

// Usage
const passwordValidator = new Validator<string>()
  .add(validators.required(), "Password is required")
  .add(validators.minLength(12), "Password must be at least 12 characters")
  .add(
    validators.pattern(/[A-Z]/, "Must contain uppercase"),
    "Must contain uppercase",
  )
  .add(
    validators.pattern(/[a-z]/, "Must contain lowercase"),
    "Must contain lowercase",
  )
  .add(
    validators.pattern(/[0-9]/, "Must contain number"),
    "Must contain number",
  )
  .add(
    validators.custom(
      (v) => !["password123", "qwerty123"].includes(v.toLowerCase()),
      "Password is too common",
    ),
    "Password is too common",
  );

const result = passwordValidator.validate("MyPass123!");

// Async validators
type AsyncValidatorFn<T> = (value: T) => Promise<boolean | string>;

class AsyncValidator<T> {
  private validators: Array<{ fn: AsyncValidatorFn<T>; message: string }> = [];

  add(fn: AsyncValidatorFn<T>, message: string): this {
    this.validators.push({ fn, message });
    return this;
  }

  async validate(value: T): Promise<{ valid: boolean; errors: string[] }> {
    const errors: string[] = [];

    const results = await Promise.all(
      this.validators.map(async ({ fn, message }) => {
        try {
          const result = await fn(value);
          if (result === false) return message;
          if (typeof result === "string") return result;
          return null;
        } catch {
          return message;
        }
      }),
    );

    for (const error of results) {
      if (error) errors.push(error);
    }

    return { valid: errors.length === 0, errors };
  }
}

// Async validator example
const emailValidator = new AsyncValidator<string>()
  .add(async (email) => {
    const exists = await db.users.findByEmail(email);
    return !exists;
  }, "Email already registered")
  .add(async (email) => {
    // Check against disposable email domains
    const domain = email.split("@")[1];
    const isDisposable = await checkDisposableDomain(domain);
    return !isDisposable;
  }, "Disposable email addresses are not allowed");
```

### Validation Error Handling

```typescript
// Structured validation error
interface ValidationError {
  field: string;
  message: string;
  code: string;
  value?: unknown;
}

class ValidationException extends Error {
  public readonly errors: ValidationError[];
  public readonly statusCode = 422;

  constructor(errors: ValidationError[]) {
    super("Validation failed");
    this.name = "ValidationException";
    this.errors = errors;
  }

  toJSON() {
    return {
      error: "Validation Error",
      message: this.message,
      details: this.errors,
    };
  }

  static single(
    field: string,
    message: string,
    code: string = "invalid",
  ): ValidationException {
    return new ValidationException([{ field, message, code }]);
  }

  static fromZod(error: ZodError): ValidationException {
    return new ValidationException(
      error.errors.map((e) => ({
        field: e.path.join("."),
        message: e.message,
        code: e.code,
      })),
    );
  }
}

// Express error handler middleware
function validationErrorHandler(
  err: Error,
  req: Request,
  res: Response,
  next: NextFunction,
): void {
  if (err instanceof ValidationException) {
    res.status(err.statusCode).json(err.toJSON());
    return;
  }

  if (err instanceof ZodError) {
    const validationError = ValidationException.fromZod(err);
    res.status(validationError.statusCode).json(validationError.toJSON());
    return;
  }

  next(err);
}

// Field-level error accumulator
class ValidationErrorCollector {
  private errors: Map<string, string[]> = new Map();

  addError(field: string, message: string): void {
    const existing = this.errors.get(field) || [];
    this.errors.set(field, [...existing, message]);
  }

  hasErrors(): boolean {
    return this.errors.size > 0;
  }

  getErrors(): Record<string, string[]> {
    return Object.fromEntries(this.errors);
  }

  getFirstErrors(): Record<string, string> {
    const result: Record<string, string> = {};
    for (const [field, messages] of this.errors) {
      result[field] = messages[0];
    }
    return result;
  }

  toException(): ValidationException {
    const errors: ValidationError[] = [];
    for (const [field, messages] of this.errors) {
      for (const message of messages) {
        errors.push({ field, message, code: "validation_error" });
      }
    }
    return new ValidationException(errors);
  }

  throwIfErrors(): void {
    if (this.hasErrors()) {
      throw this.toException();
    }
  }
}

// Usage
const collector = new ValidationErrorCollector();

if (!isValidEmail(data.email)) {
  collector.addError("email", "Invalid email format");
}

if (data.password.length < 12) {
  collector.addError("password", "Password must be at least 12 characters");
}

if (data.password !== data.confirmPassword) {
  collector.addError("confirmPassword", "Passwords do not match");
}

collector.throwIfErrors();
```

## Best Practices

1. **Validate Early**
   - Validate at the boundary (API endpoints, form submissions)
   - Fail fast with clear error messages
   - Don't trust any external input

2. **Use Schema Validation Libraries**
   - Prefer Zod/Pydantic for type safety
   - JSON Schema for language-agnostic validation
   - Generate TypeScript types from schemas

3. **Sanitize and Encode**
   - Sanitize input based on context (HTML, SQL, paths)
   - Encode output based on where it's rendered
   - Use parameterized queries instead of escaping for SQL

4. **Error Messages**
   - Provide specific, actionable error messages
   - Include field names in errors
   - Don't expose internal details in production

5. **Defense in Depth**
   - Validate on both client and server
   - Apply principle of least privilege
   - Whitelist rather than blacklist

## Examples

### Express Validation Middleware

```typescript
import { Request, Response, NextFunction } from "express";
import { z, ZodSchema } from "zod";

function validate<T>(
  schema: ZodSchema<T>,
  source: "body" | "query" | "params" = "body",
) {
  return (req: Request, res: Response, next: NextFunction) => {
    const result = schema.safeParse(req[source]);

    if (!result.success) {
      return res.status(422).json({
        error: "Validation Error",
        details: result.error.errors.map((e) => ({
          field: e.path.join("."),
          message: e.message,
        })),
      });
    }

    req[source] = result.data;
    next();
  };
}

// Usage
const createUserSchema = z.object({
  email: z.string().email(),
  password: z.string().min(12),
  name: z.string().min(1).max(100),
});

app.post("/users", validate(createUserSchema), async (req, res) => {
  // req.body is now typed and validated
  const user = await createUser(req.body);
  res.status(201).json(user);
});
```
