---
name: prompt-engineering
description: Designs and optimizes prompts for large language models to achieve better, more consistent outputs. Trigger keywords: prompt, LLM, GPT, Claude, prompt engineering, AI prompts, few-shot, chain of thought.
allowed-tools: Read, Grep, Glob, Edit, Write
---

# Prompt Engineering

## Overview

This skill focuses on crafting effective prompts for large language models. It covers techniques for improving output quality, consistency, and reliability across various use cases.

## Instructions

### 1. Define the Task Clearly

- Identify the specific goal
- Determine output format requirements
- Consider edge cases
- Plan for error handling

### 2. Structure the Prompt

- Use clear, specific instructions
- Provide relevant context
- Include examples when helpful
- Specify constraints and format

### 3. Apply Techniques

- Chain of thought reasoning
- Few-shot learning
- Role prompting
- Output formatting

### 4. Iterate and Refine

- Test with diverse inputs
- Analyze failure cases
- Optimize for consistency
- Document effective patterns

## Best Practices

1. **Be Specific**: Vague prompts yield vague results
2. **Provide Context**: Give necessary background information
3. **Show Examples**: Demonstrate desired output format
4. **Constrain Output**: Specify format, length, style
5. **Think Step by Step**: Break complex tasks into steps
6. **Test Edge Cases**: Verify behavior with unusual inputs
7. **Version Control**: Track prompt iterations

## Examples

### Example 1: Basic Prompt Structure

```markdown
# Poor Prompt

Summarize this article.

# Good Prompt

You are an expert technical writer. Summarize the following article for a software engineering audience.

## Requirements:

- Length: 2-3 paragraphs
- Include: key findings, methodology, and practical implications
- Tone: professional and objective
- Format: plain text with no bullet points

## Article:

{article_text}

## Summary:
```

### Example 2: Few-Shot Learning

````markdown
# Task: Extract structured data from product descriptions

## Examples:

Input: "Apple MacBook Pro 14-inch with M3 chip, 16GB RAM, 512GB SSD. Space Gray. $1,999"
Output:

```json
{
  "brand": "Apple",
  "product": "MacBook Pro",
  "specs": {
    "screen_size": "14-inch",
    "processor": "M3 chip",
    "ram": "16GB",
    "storage": "512GB SSD"
  },
  "color": "Space Gray",
  "price": 1999
}
```
````

Input: "Samsung Galaxy S24 Ultra, 256GB, Titanium Black, unlocked - $1,299.99"
Output:

```json
{
  "brand": "Samsung",
  "product": "Galaxy S24 Ultra",
  "specs": {
    "storage": "256GB",
    "carrier": "unlocked"
  },
  "color": "Titanium Black",
  "price": 1299.99
}
```

Now extract data from:
Input: "{new_product_description}"
Output:

````

### Example 3: Chain of Thought Prompting
```markdown
# Task: Solve complex reasoning problems

You are a logical reasoning expert. Solve the following problem step by step.

## Problem:
A store sells apples and oranges. Apples cost $2 each and oranges cost $3 each.
If Sarah buys 12 pieces of fruit for exactly $30, how many of each did she buy?

## Solution Process:
Let me work through this systematically:

Step 1: Define variables
- Let a = number of apples
- Let o = number of oranges

Step 2: Set up equations from the constraints
- Total fruit: a + o = 12
- Total cost: 2a + 3o = 30

Step 3: Solve the system
- From equation 1: a = 12 - o
- Substitute into equation 2: 2(12 - o) + 3o = 30
- Simplify: 24 - 2o + 3o = 30
- Solve: o = 6

Step 4: Find remaining variable
- a = 12 - 6 = 6

Step 5: Verify
- 6 apples + 6 oranges = 12 fruit ✓
- 6($2) + 6($3) = $12 + $18 = $30 ✓

## Answer:
Sarah bought 6 apples and 6 oranges.
````

### Example 4: System Prompt for Code Generation

```markdown
# System Prompt for Code Assistant

You are an expert software engineer assistant. When writing code:

## Code Quality Standards:

1. Write clean, readable code with meaningful variable names
2. Include comprehensive error handling
3. Add type hints (Python) or TypeScript types
4. Follow language-specific conventions (PEP 8 for Python, ESLint for JS)
5. Include docstrings/JSDoc for public functions

## Response Format:

1. First, briefly explain your approach (2-3 sentences)
2. Then provide the code implementation
3. Finally, explain any important design decisions or trade-offs

## Constraints:

- Prefer standard library solutions over external dependencies
- Optimize for readability over cleverness
- Include input validation for public APIs
- Write testable code with dependency injection where appropriate

## When Uncertain:

- Ask clarifying questions before implementing
- State assumptions explicitly
- Offer alternative approaches if applicable

---

User: Write a function to parse and validate email addresses
```

### Example 5: Output Formatting Control

````markdown
# Task: Analyze sentiment with structured output

Analyze the sentiment of the following customer reviews. For each review, provide:

1. Sentiment classification (positive/negative/neutral)
2. Confidence score (0.0 to 1.0)
3. Key phrases that indicate the sentiment
4. Suggested response action

## Output Format (JSON):

```json
{
  "reviews": [
    {
      "id": 1,
      "text": "original review text",
      "sentiment": "positive|negative|neutral",
      "confidence": 0.95,
      "key_phrases": ["phrase1", "phrase2"],
      "action": "thank|apologize|follow_up|escalate"
    }
  ],
  "summary": {
    "total": 3,
    "positive": 1,
    "negative": 1,
    "neutral": 1,
    "average_confidence": 0.85
  }
}
```
````

## Reviews to Analyze:

{reviews_list}

## Analysis:

```

```
