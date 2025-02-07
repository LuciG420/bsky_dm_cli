#!python
import asyncio
import logging
from atproto import Client, models
from ably import Ably
import os
from dotenv import load_dotenv

class BskyXRPCDaemon:
    def __init__(self, 
                 bsky_username=None, 
                 bsky_password=None, 
                 ably_api_key=None,
                 channel_name='bsky-events'):
        """
        Initialize Bluesky XRPC daemon for event streaming
        
        :param bsky_username: Bluesky account username
        :param bsky_password: Bluesky account password
        :param ably_api_key: Ably realtime API key
        :param channel_name: Ably channel to publish events
        """
        # Load environment variables
        load_dotenv()
        
        # Credentials from environment or parameters
        self.bsky_username = bsky_username or os.getenv('BSKY_USERNAME')
        self.bsky_password = bsky_password or os.getenv('BSKY_PASSWORD')
        self.ably_api_key = ably_api_key or os.getenv('ABLY_API_KEY')
        
        # Configure logging
        logging.basicConfig(
            level=logging.INFO, 
            format='%(asctime)s - %(levelname)s: %(message)s'
        )
        self.logger = logging.getLogger(__name__)
        
        # Initialize clients
        self.bsky_client = Client()
        self.ably_client = Ably.Rest(key=self.ably_api_key)
        self.channel = self.ably_client.channels.get(channel_name)
        
    async def authenticate(self):
        """Authenticate with Bluesky"""
        try:
            self.bsky_client.login(
                self.bsky_username, 
                self.bsky_password
            )
            self.logger.info("Successfully authenticated with Bluesky")
        except Exception as e:
            self.logger.error(f"Authentication failed: {e}")
            raise
    
    async def stream_posts(self):
        """Stream posts from Bluesky and publish to Ably"""
        try:
            async for post in self.bsky_client.stream.search_posts():
                event_data = {
                    'type': 'post',
                    'did': post.author.did,
                    'cid': post.cid,
                    'text': post.record.text,
                    'timestamp': post.record.created_at
                }
                
                # Publish to Ably
                self.channel.publish('new_post', event_data)
                self.logger.info(f"Published post from {post.author.did}")
        
        except Exception as e:
            self.logger.error(f"Error streaming posts: {e}")
    
    async def stream_notifications(self):
        """Stream notifications from Bluesky"""
        try:
            async for notification in self.bsky_client.stream.notifications():
                event_data = {
                    'type': 'notification',
                    'reason': notification.reason,
                    'author': notification.author.did,
                    'record': notification.record
                }
                
                # Publish to Ably
                self.channel.publish('notification', event_data)
                self.logger.info(f"Published notification for {notification.author.did}")
        
        except Exception as e:
            self.logger.error(f"Error streaming notifications: {e}")
    
    async def run(self):
        """Main daemon run method"""
        await self.authenticate()
        
        # Concurrent streams
        await asyncio.gather(
            self.stream_posts(),
            self.stream_notifications()
        )

async def main():
    daemon = BskyXRPCDaemon()
    await daemon.run()

if __name__ == '__main__':
    asyncio.run(main())
