void main() {
    vec3 color = vec3(1.0);
    for (int i = 0; i < 10; i++) {
        if (i > 5) {
            color.x += 1.0;
        } else {
            color.y += 1.0;
        }
    }
}
