function greet(name) {
  if (name) {
    return `hello ${name}`;
  } else {
    return "hello";
  }
}

const config = {
  name: "test",
  values: [1, 2, 3],
  nested: {
    enabled: true,
    handler() {
      return null;
    },
  },
};

class Counter {
  constructor(start) {
    this.count = start;
  }

  increment() {
    this.count += 1;
    return this.count;
  }

  static create() {
    return new Counter(0);
  }
}

function iterate(items) {
  const seen = [];

  for (const item of items) {
    switch (item.type) {
      case "a":
        seen.push(item);
        break;
      default:
        continue;
    }
  }

  let i = 0;
  while (i < items.length) {
    i += 1;
  }

  return seen;
}

const fetchData = async (url) => {
  try {
    const response = await fetch(url);
    return response;
  } catch (error) {
    console.error(error);
  }
};

const { first, second } = config;

setTimeout(() => {
  doSomething(() => {
    cleanup();
  });
}, 1000);

function totals() {
  return [1, 2, 3]
    .filter((n) => n > 0)
    .map((n) => n * 2)
    .reduce((a, b) => a + b, 0);
}

const scaled = [1, 2, 3]
  .filter((n) => n > 0)
  .map((n) => n * 2);

builder
  .setName("test")
  .setValue(42)
  .build();

fetch(url)
  .then((res) => res.json())
  .then((data) => {
    console.log(data);
  })
  .catch((err) => {
    handle(err);
  });

const result = create({
  name: "test",
})
  .validate()
  .save();

doSomething(
  argument1,
  argument2,
);

function braceless() {
  if (cond)
    doThing();
  else
    other();
  if (a)
    one();
  else if (b)
    two();
  while (go)
    step();
  for (let i = 0; i < n; i++)
    use(i);
  for (const x of xs)
    use(x);
  do
    once();
  while (cond);
}
