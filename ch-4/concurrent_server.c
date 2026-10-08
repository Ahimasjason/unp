

#include <common.h>

#include <stdio.h>
#include <stdlib.h>
#include<sys/socket.h>
#include <arpa/inet.h>
#include <unistd.h>
#include<errno.h>
#include <sys/wait.h>



#define HOST "0.0.0.0"
#define PORT 8080
#define BACKLOG 5


void work(int connid);
int main() {

    int sockfd, connid;
    struct sockaddr_in server_addr = {0}; 
    struct sockaddr_in client_addr = {0};
    socklen_t client_addr_len;
    pid_t clpid;
    sockfd = socket(AF_INET, SOCK_STREAM, 0);

    if (sockfd < 0) {
        err_sys("failed to create socket");
    }

    server_addr.sin_family = AF_INET;
    server_addr.sin_addr.s_addr = inet_addr(HOST);
    server_addr.sin_port = htons(PORT);

    Bind(sockfd, (struct sockaddr *)&server_addr, sizeof(server_addr));
    
    // I have to listen to the address
    Listen(sockfd, BACKLOG);
    
    client_addr_len = sizeof(client_addr);

    for (;;){
        connid = accept(sockfd, (struct sockaddr *)&client_addr, &client_addr_len);
        if (connid < 0 ) {
            if (errno == EINTR)
                continue;
            Close(sockfd);
            err_sys("failed to accept");
        }
        if ((clpid = fork()) == 0){
            // client  close the socket , listen 
            close(sockfd);
            work(connid);
            close(connid);
            exit(0);
        }

        if (clpid < 0 ){
            Close(sockfd);
            err_sys("failed to fork");
        }
            

        Close(connid);

    }
    

    // wait for the all the children to exit

    // while  (waitpid(-1, NULL, WNOHANG) > 0)  {
    //     WIFSTOPPED()
    // }

}



void work(int connid){
    char buf[1024];
    ssize_t n;
    
    // again:
    // while ((n = read(connid, buf, sizeof(buf))) > 0) {
    //     if (write(connid , buf, n) < 0 )
    //         err_sys("failed to write to the writer");
        
    // }

    // if (n < 0) {
    //     if (errno == EINTR)
    //         goto again;
    //     err_sys("read failed");
    // }



    for (;;){

        // read from the 
        n = read(connid, buf, sizeof(buf));

        if (n == 0)
            return;
        
        if ( n < 0 ){
            if (errno == EINTR)
                continue;
            
            err_sys("failed to read");
        }

        ssize_t sent = 0;

        while (sent < n) {
            ssize_t written = write(connid, buf + sent, n - sent);

            if (written < 0){

                if (errno == EINTR) 
                    continue;

                err_sys("failed to write");

            }
            if (written == 0 ){
                return;
            }
            sent += written;
        } 
    }
}