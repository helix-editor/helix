(defn process [items]
  (reduce + 0 items))

(let [a 1
      b 2]
  (+ a b))

(loop [i 0]
  (recur (inc i)))

(println "first"
         "aligned-to-first")
