# frozen_string_literal: true

# config/application.rb already loads active_job. Configure adapter.
# In config/environments/development.rb and production.rb set:
# config.active_job.queue_adapter = :sidekiq

Rails.application.config.active_job.queue_adapter = :sidekiq
