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
#include <queue>
#include <sys/epoll.h>

void serve_connection(int sockfd)
{
    if (send(sockfd, "HTTP/1.1 200 OK\r\n", 17, 0) < 0)
    {
        perror("send");
        return;
    }
    char buf[4096];
    // std::cout << "meows: ";
    // std::string tmp;
    // std::cin >> tmp;
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
void set_non_blocking(int sockfd)
{
    int flags = fcntl(sockfd, F_GETFL, 0);
    if (flags < 0)
    {
        perror("fcntl");
        return;
    }
    if (fcntl(sockfd, F_SETFL, flags | O_NONBLOCK) < 0)
    {
        perror("fcntl");
        return;
    }
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
    set_non_blocking(sockfd);

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
    int kdpfd = epoll_create(10);
    if (kdpfd < 0)
    {
        perror("epoll_create");
        return 1;
    }
    struct epoll_event ev;
    ev.events = EPOLLIN | EPOLLET;
    ev.data.fd = sockfd;
    if (epoll_ctl(kdpfd, EPOLL_CTL_ADD, sockfd, &ev) < 0)
    {
        perror("epoll_ctl");
        return 1;
    }
    struct epoll_event *events;
    events = (epoll_event *)calloc(10, sizeof(struct epoll_event));
    if (events == NULL)
    {
        perror("calloc");
        return 1;
    }

    while (1)
    {
        int nfds = epoll_wait(kdpfd, events, 10, -1);
        if (nfds < 0)
        {
            perror("epoll_wait");
            return 1;
        }
        for (int i = 0; i < nfds; i++)
        {
            if (events[i].data.fd == sockfd)
            {
                struct sockaddr_in peer_addr;
                socklen_t peer_addr_len = sizeof(peer_addr);
                int peer_sockfd = accept(sockfd, (struct sockaddr *)&peer_addr, &peer_addr_len);
                if (peer_sockfd < 0)
                {
                    perror("accept");
                    return 1;
                }
                set_non_blocking(peer_sockfd);
                ev.events = EPOLLIN | EPOLLET;
                ev.data.fd = peer_sockfd;
                if (epoll_ctl(kdpfd, EPOLL_CTL_ADD, peer_sockfd, &ev) < 0)
                {
                    perror("epoll_ctl");
                    return 1;
                }
            }
            else
            {
                serve_connection(events[i].data.fd);
            }
        }
    }

    return 0;
}