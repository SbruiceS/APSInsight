# frozen_string_literal: true

module Api
  module V1
    class SoilSamplesController < ApplicationController
      def index
        samples = SoilSample.includes(:field, :plot, :dataset).order(sampled_at: :desc)
        render json: samples.as_json(include: [:field, :plot, :dataset])
      end

      def show
        sample = SoilSample.find(params[:id])
        render json: sample.as_json(include: [:field, :plot, :dataset, :observations])
      end

      def create
        sample = SoilSample.new(soil_sample_params)
        if sample.save
          AuditEvent.create!(action: "create", resource_type: "SoilSample", resource_id: sample.id)
          render json: sample, status: :created
        else
          render json: { errors: sample.errors.full_messages }, status: :unprocessable_entity
        end
      end

      private

      def soil_sample_params
        params.require(:soil_sample).permit(
          :field_id, :plot_id, :dataset_id, :sampled_at, :depth_cm,
          :sample_code, :latitude, :longitude, metadata: {}
        )
      end
    end
  end
end
