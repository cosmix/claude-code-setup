---
name: typescript
description: TypeScript language expertise for writing type-safe, production-quality TypeScript code. Use for TypeScript development, advanced type system features, strict mode, and common patterns. Triggers: typescript, ts, tsx, type, generic, interface, tsconfig, discriminated union, utility types.
---

# TypeScript Language Expertise

## Overview

This skill provides guidance for writing type-safe, maintainable, and production-quality TypeScript code. It covers TypeScript's advanced type system features, strict mode configuration, module systems, and common design patterns.

## Key Concepts

### Generics

```typescript
// Basic generics
function identity<T>(value: T): T {
  return value;
}

// Multiple type parameters
function map<T, U>(items: T[], fn: (item: T) => U): U[] {
  return items.map(fn);
}

// Generic constraints
interface HasLength {
  length: number;
}

function logLength<T extends HasLength>(item: T): void {
  console.log(item.length);
}

// Generic classes
class Repository<T extends { id: string }> {
  private items: Map<string, T> = new Map();

  save(item: T): void {
    this.items.set(item.id, item);
  }

  findById(id: string): T | undefined {
    return this.items.get(id);
  }

  findAll(): T[] {
    return Array.from(this.items.values());
  }
}

// Default type parameters
interface ApiResponse<T = unknown> {
  data: T;
  status: number;
  message: string;
}
```

### Utility Types

```typescript
// Built-in utility types
interface User {
  id: string;
  email: string;
  name: string;
  role: "admin" | "user";
  createdAt: Date;
}

// Partial - all properties optional
type UserUpdate = Partial<User>;

// Required - all properties required
type RequiredUser = Required<User>;

// Readonly - all properties readonly
type ImmutableUser = Readonly<User>;

// Pick - select specific properties
type UserCredentials = Pick<User, "email" | "id">;

// Omit - exclude specific properties
type UserWithoutDates = Omit<User, "createdAt">;

// Record - create object type with specific keys
type UserRoles = Record<string, "admin" | "user" | "guest">;

// Extract/Exclude for union types
type StringOrNumber = string | number | boolean;
type OnlyStrings = Extract<StringOrNumber, string>; // string
type NoStrings = Exclude<StringOrNumber, string>; // number | boolean

// ReturnType and Parameters
function createUser(name: string, email: string): User {
  return {
    id: crypto.randomUUID(),
    name,
    email,
    role: "user",
    createdAt: new Date(),
  };
}

type CreateUserReturn = ReturnType<typeof createUser>; // User
type CreateUserParams = Parameters<typeof createUser>; // [string, string]

// NonNullable
type MaybeString = string | null | undefined;
type DefiniteString = NonNullable<MaybeString>; // string
```

### Conditional Types

```typescript
// Basic conditional type
type IsString<T> = T extends string ? true : false;

// Infer keyword for type extraction
type UnwrapPromise<T> = T extends Promise<infer U> ? U : T;
type UnwrapArray<T> = T extends (infer U)[] ? U : T;

// Nested inference
type GetReturnType<T> = T extends (...args: any[]) => infer R ? R : never;

// Distributive conditional types
type ToArray<T> = T extends any ? T[] : never;
type StringOrNumberArray = ToArray<string | number>; // string[] | number[]

// Non-distributive conditional types
type ToArrayNonDist<T> = [T] extends [any] ? T[] : never;
type Combined = ToArrayNonDist<string | number>; // (string | number)[]

// Practical example: Extract function parameters
type FirstParameter<T> = T extends (first: infer F, ...args: any[]) => any
  ? F
  : never;
```

### Mapped Types

```typescript
// Basic mapped type
type Nullable<T> = {
  [K in keyof T]: T[K] | null;
};

// With modifiers
type Mutable<T> = {
  -readonly [K in keyof T]: T[K];
};

type Optional<T> = {
  [K in keyof T]+?: T[K];
};

// Key remapping (TypeScript 4.1+)
type Getters<T> = {
  [K in keyof T as `get${Capitalize<string & K>}`]: () => T[K];
};

type Setters<T> = {
  [K in keyof T as `set${Capitalize<string & K>}`]: (value: T[K]) => void;
};

// Filter keys
type FilterByType<T, U> = {
  [K in keyof T as T[K] extends U ? K : never]: T[K];
};

interface Mixed {
  name: string;
  age: number;
  active: boolean;
  email: string;
}

type StringProps = FilterByType<Mixed, string>; // { name: string; email: string }

// Practical: API response transformation
type ApiDTO<T> = {
  [K in keyof T as `${string & K}DTO`]: T[K] extends Date ? string : T[K];
};
```

### Discriminated Unions

```typescript
// Define discriminated union with literal type discriminator
type Result<T, E = Error> =
  | { success: true; data: T }
  | { success: false; error: E };

function handleResult<T>(result: Result<T>): T {
  if (result.success) {
    return result.data; // TypeScript knows data exists here
  }
  throw result.error; // TypeScript knows error exists here
}

// More complex example: State machine
type LoadingState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "success"; data: User[] }
  | { status: "error"; error: Error };

function renderState(state: LoadingState): string {
  switch (state.status) {
    case "idle":
      return "Click to load";
    case "loading":
      return "Loading...";
    case "success":
      return `Loaded ${state.data.length} users`;
    case "error":
      return `Error: ${state.error.message}`;
  }
}

// Action types for Redux-style reducers
type Action =
  | { type: "SET_USER"; payload: User }
  | { type: "CLEAR_USER" }
  | { type: "SET_ERROR"; payload: string };

function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "SET_USER":
      return { ...state, user: action.payload };
    case "CLEAR_USER":
      return { ...state, user: null };
    case "SET_ERROR":
      return { ...state, error: action.payload };
  }
}
```

### Type Guards

```typescript
// typeof guard
function process(value: string | number): string {
  if (typeof value === "string") {
    return value.toUpperCase();
  }
  return value.toFixed(2);
}

// instanceof guard
function handleError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

// Custom type guard
interface Cat {
  meow(): void;
}

interface Dog {
  bark(): void;
}

function isCat(animal: Cat | Dog): animal is Cat {
  return "meow" in animal;
}

// Type guard with discriminated unions
function isSuccess<T>(result: Result<T>): result is { success: true; data: T } {
  return result.success;
}

// Assertion function
function assertNonNull<T>(
  value: T | null | undefined,
  message?: string,
): asserts value is T {
  if (value === null || value === undefined) {
    throw new Error(message ?? "Value is null or undefined");
  }
}

// Usage
function processUser(user: User | null) {
  assertNonNull(user, "User must exist");
  // user is now User (not null)
  console.log(user.name);
}
```

## Best Practices

### Strict Mode Configuration

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "lib": ["ES2022"],
    "outDir": "./dist",
    "rootDir": "./src",
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,

    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "exactOptionalPropertyTypes": true,

    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "isolatedModules": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
```

### Module Organization

```typescript
// Re-export pattern for clean public API
// src/models/index.ts
export { User, type UserDTO } from "./user";
export { Order, type OrderDTO } from "./order";
export { Product, type ProductDTO } from "./product";

// Barrel exports with explicit types
// src/index.ts
export type { Config, ConfigOptions } from "./config";
export { createConfig, validateConfig } from "./config";

// Namespace imports for related utilities
import * as validators from "./validators";
import * as formatters from "./formatters";

// Type-only imports
import type { User, Order } from "./models";
import { createUser, createOrder } from "./models";
```

### Declaration Files

```typescript
// global.d.ts - Extend global types
declare global {
  interface Window {
    analytics: AnalyticsAPI;
  }

  namespace NodeJS {
    interface ProcessEnv {
      NODE_ENV: "development" | "production" | "test";
      DATABASE_URL: string;
      API_KEY: string;
    }
  }
}

// module.d.ts - Declare untyped modules
declare module "untyped-package" {
  export function doSomething(value: string): void;
  export const VERSION: string;
}

// Augment existing modules
declare module "express" {
  interface Request {
    user?: User;
    requestId: string;
  }
}

export {}; // Makes this a module
```

## Common Patterns

### Branded Types

```typescript
// Create nominal types for type safety
declare const brand: unique symbol;

type Brand<T, B> = T & { [brand]: B };

type UserId = Brand<string, "UserId">;
type OrderId = Brand<string, "OrderId">;
type Email = Brand<string, "Email">;

// Constructor functions with validation
function createUserId(id: string): UserId {
  if (!id.match(/^usr_[a-z0-9]+$/)) {
    throw new Error("Invalid user ID format");
  }
  return id as UserId;
}

function createEmail(email: string): Email {
  if (!email.includes("@")) {
    throw new Error("Invalid email format");
  }
  return email.toLowerCase() as Email;
}

// Now these can't be accidentally mixed
function getUser(id: UserId): Promise<User> {
  /* ... */
}
function getOrder(id: OrderId): Promise<Order> {
  /* ... */
}

// const userId = createUserId('usr_123');
// const orderId = createOrderId('ord_456');
// getUser(orderId); // Type error!
```

### Builder Pattern

```typescript
class QueryBuilder<T extends object> {
  private filters: Partial<T> = {};
  private sortField?: keyof T;
  private sortOrder: "asc" | "desc" = "asc";
  private limitValue?: number;
  private offsetValue?: number;

  where<K extends keyof T>(field: K, value: T[K]): this {
    this.filters[field] = value;
    return this;
  }

  orderBy(field: keyof T, order: "asc" | "desc" = "asc"): this {
    this.sortField = field;
    this.sortOrder = order;
    return this;
  }

  limit(value: number): this {
    this.limitValue = value;
    return this;
  }

  offset(value: number): this {
    this.offsetValue = value;
    return this;
  }

  build(): Query<T> {
    return {
      filters: this.filters,
      sort: this.sortField
        ? { field: this.sortField, order: this.sortOrder }
        : undefined,
      pagination: { limit: this.limitValue, offset: this.offsetValue },
    };
  }
}

// Usage with type inference
const query = new QueryBuilder<User>()
  .where("role", "admin")
  .orderBy("createdAt", "desc")
  .limit(10)
  .build();
```

### Exhaustive Checks

```typescript
// Ensure all union cases are handled
function assertNever(value: never): never {
  throw new Error(`Unexpected value: ${value}`);
}

type Status = "pending" | "approved" | "rejected" | "cancelled";

function getStatusColor(status: Status): string {
  switch (status) {
    case "pending":
      return "yellow";
    case "approved":
      return "green";
    case "rejected":
      return "red";
    case "cancelled":
      return "gray";
    default:
      return assertNever(status); // Compile error if case is missing
  }
}

// With discriminated unions
type Event =
  | { type: "click"; x: number; y: number }
  | { type: "keypress"; key: string }
  | { type: "scroll"; delta: number };

function handleEvent(event: Event): void {
  switch (event.type) {
    case "click":
      console.log(`Clicked at ${event.x}, ${event.y}`);
      break;
    case "keypress":
      console.log(`Key pressed: ${event.key}`);
      break;
    case "scroll":
      console.log(`Scrolled: ${event.delta}`);
      break;
    default:
      assertNever(event);
  }
}
```

### Type-Safe Event Emitter

```typescript
type EventMap = {
  userCreated: { user: User };
  userDeleted: { userId: string };
  orderPlaced: { order: Order; user: User };
};

class TypedEventEmitter<T extends Record<string, any>> {
  private listeners: { [K in keyof T]?: Array<(payload: T[K]) => void> } = {};

  on<K extends keyof T>(
    event: K,
    listener: (payload: T[K]) => void,
  ): () => void {
    if (!this.listeners[event]) {
      this.listeners[event] = [];
    }
    this.listeners[event]!.push(listener);

    return () => this.off(event, listener);
  }

  off<K extends keyof T>(event: K, listener: (payload: T[K]) => void): void {
    const listeners = this.listeners[event];
    if (listeners) {
      const index = listeners.indexOf(listener);
      if (index !== -1) {
        listeners.splice(index, 1);
      }
    }
  }

  emit<K extends keyof T>(event: K, payload: T[K]): void {
    this.listeners[event]?.forEach((listener) => listener(payload));
  }
}

// Usage
const emitter = new TypedEventEmitter<EventMap>();

emitter.on("userCreated", ({ user }) => {
  console.log(`User created: ${user.name}`);
});

emitter.emit("userCreated", { user: newUser });
// emitter.emit('userCreated', { wrong: 'payload' }); // Type error!
```

## Anti-Patterns

### Avoid These Practices

```typescript
// BAD: Using `any` to bypass type checking
function process(data: any): any {
  return data.foo.bar.baz;
}

// GOOD: Use unknown and narrow the type
function process(data: unknown): string {
  if (isValidData(data)) {
    return data.foo.bar.baz;
  }
  throw new Error("Invalid data");
}

// BAD: Type assertions without validation
const user = JSON.parse(input) as User;

// GOOD: Validate at runtime (use zod, io-ts, etc.)
import { z } from "zod";

const UserSchema = z.object({
  id: z.string(),
  email: z.string().email(),
  name: z.string(),
});

const user = UserSchema.parse(JSON.parse(input));

// BAD: Non-null assertion operator abuse
function getUser(id: string): User {
  return users.find((u) => u.id === id)!; // Crashes if not found
}

// GOOD: Handle the undefined case
function getUser(id: string): User | undefined {
  return users.find((u) => u.id === id);
}

// Or throw explicitly
function getUser(id: string): User {
  const user = users.find((u) => u.id === id);
  if (!user) {
    throw new Error(`User not found: ${id}`);
  }
  return user;
}

// BAD: Overly permissive function signatures
function merge(a: object, b: object): object {
  return { ...a, ...b };
}

// GOOD: Use generics to preserve types
function merge<T extends object, U extends object>(a: T, b: U): T & U {
  return { ...a, ...b };
}

// BAD: Using enums (they have runtime overhead and quirks)
enum Status {
  Pending,
  Active,
  Completed,
}

// GOOD: Use const objects or union types
const Status = {
  Pending: "pending",
  Active: "active",
  Completed: "completed",
} as const;

type Status = (typeof Status)[keyof typeof Status];

// BAD: Interface merging by accident
interface Config {
  port: number;
}

interface Config {
  host: string;
}
// Now Config has both port and host - often unintentional

// GOOD: Use type aliases when you don't want merging
type Config = {
  port: number;
  host: string;
};

// BAD: Ignoring strictNullChecks issues
function getLength(str: string | null): number {
  return str.length; // Runtime error if null
}

// GOOD: Proper null handling
function getLength(str: string | null): number {
  return str?.length ?? 0;
}
```
