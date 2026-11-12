module Api
  module V1
    class HealthController < ApplicationController
      def live
        render json: { status: 'ok', timestamp: Time.current.iso8601 }
      end

      def ready
        # Add database connectivity check when models exist
        render json: { status: 'ready', database: 'pending' }
      end

      def capabilities
        render json: {
          platform: 'APSInsight',
          version: '0.1.0',
          languages: ['ruby', 'typescript', 'python', 'cpp', 'r'],
          features: []
        }
      end
    end
  end
end
