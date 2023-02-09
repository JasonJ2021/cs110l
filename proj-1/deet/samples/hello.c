#include <stdio.h>

void foo(){
    printf("Hello World\n");
}

int main() {
    int n = 9;
    while (n--){
        foo();
    }
}

