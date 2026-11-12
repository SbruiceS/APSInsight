# frozen_string_literal: true

module Api
  module V1
    class ObservationsController < ApplicationController
      def index
        observations = Observation.includes(:soil_sample, :dataset).order(observed_at: :desc).limit(500)
        render json: observations.as_json(include: [:soil_sample, :dataset])
      end

      def show
        render json: Observation.find(params[:id]).as_json(include: [:soil_sample, :dataset])
      end

      def create
        observation = Observation.new(observation_params)
        if observation.save
          AuditEvent.create!(action: "create", resource_type: "Observation", resource_id: observation.id)
          render json: observation, status: :created
        else
          render json: { errors: observation.errors.full_messages }, status: :unprocessable_entity
        end
      end

      private

      def observation_params
        params.require(:observation).permit(
          :soil_sample_id, :dataset_id, :variable, :value, :unit,
          :observed_at, :quality_flag, :methodology, uncertainty: {}
        )
      end
    end
  end
end
