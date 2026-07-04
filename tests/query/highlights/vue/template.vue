<template>
  <section class="box">
<!--       ^ @attribute -->
<!--              ^ @string -->
    <MyComp :prop="x" @click="go" v-if="ok">{{ count }}</MyComp>
<!-- ^ @tag -->
<!--         ^ @attribute -->
<!--               ^ @variable -->
<!--                          ^ @variable -->
<!--                                           ^ @variable -->
<!--                                                     ^ @tag -->
  </section>
<!--^ @tag -->
</template>
