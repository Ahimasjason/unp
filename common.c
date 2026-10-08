#include "common.h"

#include <sys/socket.h>
#include <errno.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>


void err_sys(const char *fmt, ...)
{

    fprintf(stderr, "%s\t%s\n", strerror(errno), fmt);
    exit(EXIT_FAILURE);
}
 


void Listen(int sockfd, int backlog)
{

     if (listen(sockfd, backlog) != 0)
        err_sys("failed to listen");

}

void Bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen)
{

    if (bind(sockfd, addr, addrlen) != 0)
        err_sys("failed to bind");

}



void Close(int fd)
{
    if (close(fd) != 0)
        err_sys("failed to close");
}