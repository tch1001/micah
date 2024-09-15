// #include "src/main.rs.h"
// #include "rust/cxx.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

#include <fcntl.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <netdb.h>

void serve_connection(int sockfd)
{
    if (send(sockfd, "HTTP/1.1 200 OK\r\n", 17, 0) < 0)
    {
        perror("send");
        return;
    }
    while (true)
    {
        char buf[4096];
        ssize_t n = recv(sockfd, buf, sizeof(buf), 0);
        if (n < 0)
        {
            perror("recv");
            return;
        }
        else if (n == 0)
        {
            break;
        }

        /*
        GET /0x123 HTTP/1.1
        Host: localhost:8081
        User-Agent: curl/7.71.1
        Accept:
        */

        for (int i = 0; i < n; i++)
        {
            printf("%c", buf[i]);
        }
        printf("newline\n");
        // process_tx
        const std::string tx_str = "0xe8c208398bd5ae8e4c237658580db56a2a94dfa0ca382c99b776fa6e7d31d5b4";
        double tx_fee;
        rust::Str tx_str_rust(tx_str);
        // tx_fee = process_tx(tx_str_rust);
        tx_fee = 0.1;

        char response[1024];
        snprintf(response, sizeof(response), "HTTP/1.1 200 OK\r\nContent-Length: %d\r\n\r\n%.2f", (int)strlen(response), tx_fee);

        send(sockfd, response, sizeof(response), 0);
    }
    close(sockfd);
}

int main()
{
    woof();
    setvbuf(stdout, NULL, _IONBF, 0);

    int sockfd = socket(AF_INET, SOCK_STREAM, 0);
    if (sockfd < 0)
    {
        perror("socket");
        return 1;
    }

    int opt = 1;
    if (setsockopt(sockfd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)) < 0)
    {
        perror("setsockopt");
        return 1;
    }

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(8081);
    addr.sin_addr.s_addr = INADDR_ANY;

    if (bind(sockfd, (struct sockaddr *)&addr, sizeof(addr)) < 0)
    {
        perror("bind");
        return 1;
    }

    if (listen(sockfd, 10) < 0)
    {
        perror("listen");
        return 1;
    }
    while (true)
    {
        struct sockaddr_in peer_addr;
        socklen_t peer_addr_len = sizeof(peer_addr);
        int peer_sockfd = accept(sockfd, (struct sockaddr *)&peer_addr, &peer_addr_len);
        if (peer_sockfd < 0)
        {
            perror("accept");
            return 1;
        }
        serve_connection(peer_sockfd);
    }

    return 0;
}