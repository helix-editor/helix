<template>
  <div class="container">
    <ul>
      <li>{{ item }}</li>
    </ul>
    <img src="x.png" />
    <MyComponent
      :prop="value"
      @click="handler"
    />
  </div>
</template>

<script>
export default {
  data() {
    return { count: 0 };
  },
};
</script>

<style scoped>
.container {
  color: red;
}
</style>
