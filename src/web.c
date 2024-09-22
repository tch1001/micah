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
#include <newlib/sys/select.h>
#include <iostream>

void serve_connection(int sockfd)
{
    if (send(sockfd, "HTTP/1.1 200 OK\r\n", 17, 0) < 0)
    {
        perror("send");
        return;
    }
    char buf[4096];
    std::cout << "receiving for " << sockfd << std::endl;
    ssize_t n = recv(sockfd, buf, sizeof(buf), 0);
    if (n < 0)
    {
        perror("recv");
        return;
    }
    else if (n == 0)
    {
        return;
    }

    /*
    GET /0x123 HTTP/1.1
    Host: localhost:8081
    User-Agent: curl/7.71.1
    Accept:
    */

    for (int i = 0; i < n; i++)
    {
        std::cout << buf[i];
    }
    std::cout << "newline" << std::endl;

    char response[1024];
    // snprintf(response, sizeof(response), "HTTP/1.1 200 OK\r\nContent-Length: %d\r\n\r\n%.2f", (int)strlen(response), tx_fee);
    strncpy(response, "HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello", sizeof(response));

    send(sockfd, response, sizeof(response), 0);
}

int main()
{
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
    fd_set master_set, working_set;
    FD_ZERO(&master_set); // Clear the master set
    int max_sd = sockfd;
    FD_SET(sockfd, &master_set); // Add the listener to the master set
    struct timeval timeout;
    timeout.tv_sec = 3;
    timeout.tv_usec = 0;

    while (1)
    {
        struct sockaddr_in peer_addr;
        socklen_t peer_addr_len = sizeof(peer_addr);
        // int peer_sockfd = accept(sockfd, (struct sockaddr *)&peer_addr, &peer_addr_len);
        memcpy(&working_set, &master_set, sizeof(master_set));
        int desc_ready = select(max_sd + 1, &working_set, NULL, NULL, &timeout);
        if (desc_ready != 0)
        {
            printf("desc_ready: %d\n", desc_ready);
        }

        if (desc_ready < 0)
        {
            perror("select() failed");
            return 1;
        }

        for (int i = 0; i <= max_sd and desc_ready > 0; i++)
        {
            if (FD_ISSET(i, &working_set))
            {
                --desc_ready;
                if (i == sockfd)
                {
                    int new_sock = accept(sockfd, (struct sockaddr *)&peer_addr, &peer_addr_len);
                    if (new_sock < 0)
                    {
                        perror("accept");
                        return 1;
                    }
                    FD_SET(new_sock, &master_set);
                    if (new_sock > max_sd)
                    {
                        max_sd = new_sock;
                    }
                }
                else
                {
                    serve_connection(i);
                }
            }
        }
    }

    return 0;
}