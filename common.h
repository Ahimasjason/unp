#ifndef UNP_COMMON_H
#define UNP_COMMON_H

#include <sys/socket.h>


void err_sys(const char *fmt, ...);
void Listen(int sockfd, int backlog);
void Close(int fd);
void Bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen);


#endif