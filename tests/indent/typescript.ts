function classify(value: number): string {
  switch (value) {
    case 1:
      return "one";
    case 2:
      console.log("two");
      return "";
    default:
      return "other";
  }
}

interface Shape {
  readonly name: string;
  area(): number;
}

type Handler = (event: string) => void;

enum Direction {
  Up,
  Down,
}

class Rectangle implements Shape {
  constructor(
    private width: number,
    private height: number,
  ) {}

  get name(): string {
    return "rectangle";
  }

  area(): number {
    return this.width * this.height;
  }
}

function process<T>(items: T[]): T[] {
  const result: T[] = [];

  const config = {
    name: "test",
    nested: {
      value: 1,
    },
  };

  for (const item of items) {
    if (item) {
      result.push(item);
    }
  }

  try {
    doSomething();
  } catch (error) {
    handle(error);
  } finally {
    cleanup();
  }

  return result;
}

const handler: Handler = (event) => {
  console.log(event);
};

function totals(): number {
  return [1, 2, 3]
    .filter((n) => n > 0)
    .map((n) => n * 2)
    .reduce((a, b) => a + b, 0);
}

function template(): string {
  const html = `
  <div>
    content
  </div>
`;
  return html;
}
