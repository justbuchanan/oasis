FROM nginx:alpine

# NOTE: must build static site with hugo first, then build the docker image.

# Copy the built Hugo site from the website/public directory
COPY public/ /usr/share/nginx/html

# Copy custom nginx configuration if needed
# COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
