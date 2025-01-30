// To-Do List Manager
class TodoList {
  constructor() {
    this.tasks = [];
    this.idCounter = 1;
  }

  // Add a new task
  addTask(description) {
    if (!description) {
      console.log("Error: Task description cannot be empty.");
      return;
    }
    const task = { id: this.idCounter++, description, completed: false };
    this.tasks.push(task);
    console.log(`Task added: "${description}"`);
  }

  // Remove a task by ID
  removeTask(id) {
    const index = this.tasks.findIndex((task) => task.id === id);
    if (index === -1) {
      console.log(`Error: Task with ID ${id} not found.`);
      return;
    }
    const removed = this.tasks.splice(index, 1);
    console.log(`Task removed: "${removed[0].description}"`);
  }

  // Mark a task as completed
  completeTask(id) {
    const task = this.tasks.find((task) => task.id === id);
    if (!task) {
      console.log(`Error: Task with ID ${id} not found.`);
      return;
    }
    task.completed = true;
    console.log(`Task completed: "${task.description}"`);
  }

  // List all tasks
  listTasks() {
    if (this.tasks.length === 0) {
      console.log("No tasks available.");
      return;
    }
    console.log("\nTo-Do List:");
    this.tasks.forEach((task) => {
      const status = task.completed ? "[x]" : "[ ]";
      console.log(`${status} ID: ${task.id} - ${task.description}`);
    });
  }
}

// Menu for managing tasks
function showMenu() {
  console.log("\n--- To-Do List Menu ---");
  console.log("1. Add a Task");
  console.log("2. Remove a Task");
  console.log("3. Complete a Task");
  console.log("4. List All Tasks");
  console.log("5. Exit");
}

// Main program loop
function main() {
  const todoList = new TodoList();
  const prompt = require("prompt-sync")();

  let running = true;
  while (running) {
    showMenu();
    const choice = prompt("Enter your choice (1-5): ");

    switch (choice) {
      case "1":
        const description = prompt("Enter task description: ");
        todoList.addTask(description);
        break;
      case "2":
        const removeId = parseInt(prompt("Enter task ID to remove: "), 10);
        todoList.removeTask(removeId);
        break;
      case "3":
        const completeId = parseInt(prompt("Enter task ID to complete: "), 10);
        todoList.completeTask(completeId);
        break;
      case "4":
        todoList.listTasks();
        break;
      case "5":
        running = false;
        console.log("Goodbye!");
        break;
      default:
        console.log("Invalid choice. Please try again.");
    }
  }
}

// Run the program
main();
