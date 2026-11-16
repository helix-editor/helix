void main() {
//           ^ 4 eof
    if (a) {}
//          ^ 4
    vec3 color = vec3(1.0);
    for (int i = 0; i < 10; i++) {
//                                ^ 8 eof
        if (i > 5) {
            color.x += 1.0;
        } else {
            color.y += 1.0;
        }
    }
}
